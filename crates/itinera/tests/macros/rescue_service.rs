//! A rescue step that requests a service, declared with the macros, against the same rescue step
//! and providers written for the builder, in a synchronous workflow.

use std::num::NonZeroU32;

use itinera::error::Error;
use itinera::journey::{Contributor, InitialData, JourneyId};
use itinera::policy::{Provider, Requested};
use itinera::service::{PerAttempt, PerJourney, Service, ServiceProviderDescriptor};
use itinera::step::{
    DescribedStep, Input, Outcome, Reason, RescueFactory, RescueNeeds, RescueOutcome,
    RescueResolved, RescueStep, Step, StepName, rescue_step, step, step_name,
};
use itinera::workflow::{Workflow, WorkflowDescriptor, workflow};
use rstest::rstest;

use super::fixtures::{Journey, Orders, orders, run_created};

/// Issues the tokens of accounts, and tells who it is.
trait TokenService: Send + Sync {
    fn label(&self) -> String;

    fn token(&self, account: &str) -> Option<String>;
}

struct Vault {
    label: String,
}

impl TokenService for Vault {
    fn label(&self) -> String {
        self.label.clone()
    }

    fn token(&self, account: &str) -> Option<String> {
        (account != "blocked").then(|| format!("{account}-from-{}", self.label))
    }
}

/// Charges with the token a rescue step left, or else fails, after contributing who checked it.
#[derive(Step)]
struct Charge {
    #[input]
    token: Option<String>,
}

#[step]
impl Charge {
    fn run(
        self,
        #[service] tokens: &dyn TokenService,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        contributor.contribute("checked by", tokens.label());
        match self.token {
            Some(_) => Self::succeed(),
            None => Self::fail("token-expired"),
        }
    }
}

/// Fetches a token for the account from the service it is lent.
#[derive(RescueStep)]
struct Refresh {
    #[input]
    account: String,
}

#[rescue_step]
impl Refresh {
    fn run(
        self,
        #[service] tokens: &dyn TokenService,
        #[contributor] contributor: Contributor<'_>,
    ) -> Result<RescueOutcome, Error> {
        Ok(refreshed(&self.account, tokens, contributor))
    }
}

fn refreshed(
    account: &str,
    tokens: &dyn TokenService,
    mut contributor: Contributor<'_>,
) -> RescueOutcome {
    contributor.contribute("refreshed by", tokens.label());
    match tokens.token(account) {
        Some(token) => {
            contributor.contribute("token", token);
            RescueOutcome::success()
        }
        None => RescueOutcome::failure(Reason::new("no-token")),
    }
}

/// The vault every step without a provider of its own is lent.
fn vault(journey: &JourneyId) -> Box<dyn TokenService> {
    Box::new(Vault {
        label: format!("vault of {journey}"),
    })
}

/// The vault lent to the charge, one for each attempt.
fn charging_vault(step: StepName, attempt: NonZeroU32) -> Box<dyn TokenService> {
    Box::new(Vault {
        label: format!("{step} vault {attempt}"),
    })
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", rescue = Refresh)]
struct Vaulted;

#[workflow]
impl Vaulted {
    #[provider]
    fn tokens(&self, #[journey_id] journey: &JourneyId) -> Result<Box<dyn TokenService>, Error> {
        Ok(vault(journey))
    }

    #[provider(per_attempt, steps = ["charge"], key = "tokens", name = "charging")]
    fn charging(
        &self,
        #[step_name] step: StepName,
        #[attempt] attempt: NonZeroU32,
    ) -> Result<Box<dyn TokenService>, Error> {
        Ok(charging_vault(step, attempt))
    }

    #[id_generator]
    fn order_id(&self) -> Result<String, Error> {
        Ok("order-1".to_owned())
    }
}

const ACCOUNT: Input<String> = Input::new("account");
const TOKENS: Service<dyn TokenService> = Service::new("tokens");

struct HandRefresh<'a> {
    account: String,
    tokens: &'a dyn TokenService,
    contributor: Contributor<'a>,
}

impl RescueStep for HandRefresh<'_> {
    fn run(self) -> Result<RescueOutcome, Error> {
        Ok(refreshed(&self.account, self.tokens, self.contributor))
    }
}

struct RefreshFactory;

impl RescueFactory for RefreshFactory {
    type Step<'a> = HandRefresh<'a>;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new()
            .input(&ACCOUNT)
            .contributor()
            .service(&TOKENS)
    }

    fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<HandRefresh<'a>, Error> {
        Ok(HandRefresh {
            account: got.input(&ACCOUNT)?,
            tokens: got.service(&TOKENS)?,
            contributor: got.contributor()?,
        })
    }
}

fn hand_tokens(
    _: &Orders,
    got: Requested<'_, Orders, Provider<PerJourney>>,
) -> Result<Box<dyn TokenService>, Error> {
    Ok(vault(got.journey_id()))
}

fn hand_charging(
    _: &Orders,
    got: Requested<'_, Orders, Provider<PerAttempt>>,
) -> Result<Box<dyn TokenService>, Error> {
    Ok(charging_vault(got.step_name(), got.attempt()))
}

/// The initial data of a journey: the account to charge.
#[derive(InitialData)]
struct Account {
    account: String,
}

fn of(account: &str) -> Account {
    Account {
        account: account.to_owned(),
    }
}

fn by_macros(account: &str) -> Journey {
    run_created(Vaulted::descriptor(), |recorder| {
        Vaulted::builder()
            .initial_data(of(account))
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

fn by_the_builder(account: &str) -> Journey {
    let charging = ServiceProviderDescriptor::per_attempt("charging", &TOKENS, hand_charging)
        .step(step_name!("charge"));
    let workflow: WorkflowDescriptor<Orders> = orders()
        .step(Charge::descriptor(step_name!("charge")).rescue(RefreshFactory))
        .provider(ServiceProviderDescriptor::new(
            "tokens",
            &TOKENS,
            hand_tokens,
        ))
        .provider(charging)
        .build()
        .unwrap();
    run_created(&workflow, |recorder| {
        workflow
            .instance(Orders)
            .initial_data(of(account))
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

#[rstest]
#[case::rescued("A-7")]
#[case::not_rescued("blocked")]
fn a_rescue_steps_service_the_macros_request_lists_and_runs_as_one_written_for_the_builder(
    #[case] account: &str,
) {
    assert_eq!(by_macros(account), by_the_builder(account));
}
