//! Asynchronous rescue steps the macros describe, against the same rescue steps written for the
//! builder, in an asynchronous workflow.

use itinera::error::Error;
use itinera::journey::{Contributor, InitialData};
use itinera::mode::Asynchronous;
use itinera::step::{
    AsyncRescueFactory, AsyncRescueStep, AsyncStepReporter, DescribedStep, OptionalInput, Outcome,
    Reason, RescueNeeds, RescueOutcome, RescueResolved, RescueStep, Step, rescue_step, step,
    step_name,
};
use itinera::workflow::{Workflow, WorkflowDescriptor, workflow};
use rstest::rstest;

use super::fixtures::{Journey, Orders, async_orders, run_created_async};

/// Charges with the token a rescue step left, or else fails as the outcome says, after
/// contributing the account it charged.
#[derive(Step)]
struct Charge {
    #[input]
    outcome: String,
    #[input]
    token: Option<String>,
}

#[step]
impl Charge {
    async fn run(self, #[contributor] mut contributor: Contributor<'_>) -> Result<Outcome, Error> {
        if let Some(token) = self.token {
            contributor.contribute("charged with", token);
            return Self::succeed();
        }
        contributor.contribute("account", "A-7".to_owned());
        match self.outcome.as_str() {
            "failure" => Self::fail("token-expired"),
            "abnormal" => Err(Error::msg("the gateway crashed")),
            _ => Self::succeed(),
        }
    }
}

/// Refreshes the token of the account the failed attempt left, if it left one.
#[derive(RescueStep)]
struct Refresh {
    #[from_rescued_step("account")]
    account: Option<String>,
    #[reason]
    reason: Option<Reason>,
}

#[rescue_step]
impl Refresh {
    async fn run(
        self,
        #[contributor] mut contributor: Contributor<'_>,
        #[reporter] mut reporter: AsyncStepReporter<'_>,
    ) -> Result<RescueOutcome, Error> {
        let Some(account) = self.account else {
            return Self::fail_retriably("no-account");
        };
        reporter
            .info_with("rescuing", cause(self.reason.as_ref()))
            .await?;
        contributor.contribute("token", format!("{account}-fresh"));
        Self::succeed()
    }
}

fn cause(reason: Option<&Reason>) -> String {
    reason.map_or_else(String::new, code)
}

fn code(reason: &Reason) -> String {
    reason.code().to_owned()
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", rescue = Refresh)]
struct Refreshing;

#[workflow]
impl Refreshing {
    #[id_generator]
    fn order_id(&self) -> Result<String, Error> {
        Ok("order-1".to_owned())
    }
}

const ACCOUNT: OptionalInput<String> = OptionalInput::new("account");

struct HandRefresh<'a> {
    account: Option<String>,
    reason: Option<Reason>,
    contributor: Contributor<'a>,
    reporter: AsyncStepReporter<'a>,
}

impl AsyncRescueStep for HandRefresh<'_> {
    async fn run(mut self) -> Result<RescueOutcome, Error> {
        let Some(account) = self.account else {
            return Ok(RescueOutcome::retriable_failure(Reason::new("no-account")));
        };
        let cause = cause(self.reason.as_ref());
        self.reporter.info_with("rescuing", cause).await?;
        self.contributor
            .contribute("token", format!("{account}-fresh"));
        Ok(RescueOutcome::success())
    }
}

struct RefreshFactory;

impl AsyncRescueFactory for RefreshFactory {
    type Step<'a> = HandRefresh<'a>;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new()
            .optional_from_rescued_step(&ACCOUNT)
            .optional_reason()
            .contributor()
            .reporter()
    }

    fn build<'a>(
        &'a self,
        got: &mut RescueResolved<'a, Asynchronous>,
    ) -> Result<HandRefresh<'a>, Error> {
        Ok(HandRefresh {
            account: got.optional_from_rescued_step(&ACCOUNT)?,
            reason: got.optional_reason()?,
            contributor: got.contributor()?,
            reporter: got.reporter()?,
        })
    }
}

/// The initial data of a journey: how the charge's first attempt ends.
#[derive(InitialData)]
struct Attempt {
    outcome: String,
}

fn ending(outcome: &str) -> Attempt {
    Attempt {
        outcome: outcome.to_owned(),
    }
}

fn by_macros(outcome: &str) -> Journey {
    run_created_async(Refreshing::descriptor(), |recorder| {
        Refreshing::builder()
            .initial_data(ending(outcome))
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

fn by_the_builder(outcome: &str) -> Journey {
    let workflow: WorkflowDescriptor<Orders, Asynchronous> = async_orders()
        .step(Charge::descriptor(step_name!("charge")).rescue_async(RefreshFactory))
        .build()
        .unwrap();
    run_created_async(&workflow, |recorder| {
        workflow
            .instance(Orders)
            .initial_data(ending(outcome))
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

#[rstest]
#[case::settled("success")]
#[case::declined("failure")]
#[case::crashed("abnormal")]
fn an_asynchronous_rescue_step_the_macros_describe_lists_and_runs_as_one_written_for_the_builder(
    #[case] outcome: &str,
) {
    assert_eq!(by_macros(outcome), by_the_builder(outcome));
}
