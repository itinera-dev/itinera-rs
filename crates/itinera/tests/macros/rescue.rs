//! Rescue steps the macros describe, against the same rescue steps written for the builder, in a
//! synchronous workflow.

use itinera::error::Error;
use itinera::journey::{Contributor, InitialData};
use itinera::step::{
    DescribedStep, Input, OptionalInput, Outcome, Reason, RescueFactory, RescueNeeds,
    RescueOutcome, RescueResolved, RescueStep, Step, StepReporter, rescue_step, step, step_name,
};
use itinera::workflow::{Workflow, WorkflowDescriptor, workflow};
use rstest::rstest;

use super::fixtures::{Journey, Orders, orders, run_created};

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
    fn run(self, #[contributor] contributor: &mut Contributor) -> Result<Outcome, Error> {
        if let Some(token) = self.token {
            contributor.contribute("charged with", token);
            return Self::succeed();
        }
        contributor.contribute("account", "A-7".to_owned());
        match self.outcome.as_str() {
            "failure" => Self::fail("token-expired"),
            "busy" => Self::fail("gateway-busy"),
            "abnormal" => Err(Error::msg("the gateway crashed")),
            _ => Self::succeed(),
        }
    }
}

/// Refreshes the token of the account the failed attempt left, if it left one.
#[derive(RescueStep)]
struct Refresh {
    #[input]
    outcome: String,
    #[from_rescued_step("account")]
    account: Option<String>,
    #[reason]
    reason: Option<Reason>,
    #[error]
    error: Option<Error>,
}

#[rescue_step]
impl Refresh {
    fn run(
        self,
        #[contributor] contributor: &mut Contributor,
        #[reporter] mut reporter: StepReporter<'_>,
    ) -> Result<RescueOutcome, Error> {
        reporter.info_with("rescuing", cause(self.reason.as_ref(), self.error.as_ref()))?;
        if self.outcome == "busy" {
            return Self::fail("still-busy");
        }
        match self.account {
            Some(account) => {
                contributor.contribute("token", format!("{account}-fresh"));
                Self::succeed()
            }
            None => Self::fail("no-account"),
        }
    }
}

/// Requires the failure's reason and the account, so that an abnormal termination, which leaves
/// neither, aborts the journey.
#[derive(RescueStep)]
struct Insist {
    #[reason]
    reason: Reason,
    #[from_rescued_step]
    account: String,
}

#[rescue_step]
impl Insist {
    fn run(self, #[contributor] contributor: &mut Contributor) -> Result<RescueOutcome, Error> {
        let token = format!("{}-{}", self.account, self.reason.code());
        contributor.contribute("token", token);
        Self::succeed()
    }
}

fn cause(reason: Option<&Reason>, error: Option<&Error>) -> String {
    match (reason, error) {
        (Some(reason), _) => reason.code().to_owned(),
        (None, Some(error)) => error.to_string(),
        (None, None) => "nothing".to_owned(),
    }
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

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", rescue = Insist)]
struct Insisting;

#[workflow]
impl Insisting {
    #[id_generator]
    fn order_id(&self) -> Result<String, Error> {
        Ok("order-1".to_owned())
    }
}

const OUTCOME: Input<String> = Input::new("outcome");
const ACCOUNT: Input<String> = Input::new("account");
const ACCOUNT_LEFT: OptionalInput<String> = OptionalInput::new("account");

struct HandRefresh<'a> {
    outcome: String,
    account: Option<String>,
    reason: Option<Reason>,
    error: Option<Error>,
    contributor: &'a mut Contributor,
    reporter: StepReporter<'a>,
}

impl RescueStep for HandRefresh<'_> {
    fn run(mut self) -> Result<RescueOutcome, Error> {
        let cause = cause(self.reason.as_ref(), self.error.as_ref());
        self.reporter.info_with("rescuing", cause)?;
        if self.outcome == "busy" {
            return Ok(RescueOutcome::failure("still-busy"));
        }
        match self.account {
            Some(account) => {
                self.contributor
                    .contribute("token", format!("{account}-fresh"));
                Ok(RescueOutcome::success())
            }
            None => Ok(RescueOutcome::failure("no-account")),
        }
    }
}

struct RefreshFactory;

impl RescueFactory for RefreshFactory {
    type Step<'a> = HandRefresh<'a>;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new()
            .input(&OUTCOME)
            .optional_from_rescued_step(&ACCOUNT_LEFT)
            .optional_reason()
            .optional_error()
            .contributor()
            .reporter()
    }

    fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<HandRefresh<'a>, Error> {
        Ok(HandRefresh {
            outcome: got.input(&OUTCOME)?,
            account: got.optional_from_rescued_step(&ACCOUNT_LEFT)?,
            reason: got.optional_reason()?,
            error: got.optional_error()?,
            contributor: got.contributor()?,
            reporter: got.reporter()?,
        })
    }
}

struct HandInsist<'a> {
    reason: Reason,
    account: String,
    contributor: &'a mut Contributor,
}

impl RescueStep for HandInsist<'_> {
    fn run(self) -> Result<RescueOutcome, Error> {
        let token = format!("{}-{}", self.account, self.reason.code());
        self.contributor.contribute("token", token);
        Ok(RescueOutcome::success())
    }
}

struct InsistFactory;

impl RescueFactory for InsistFactory {
    type Step<'a> = HandInsist<'a>;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new()
            .reason()
            .from_rescued_step(&ACCOUNT)
            .contributor()
    }

    fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<HandInsist<'a>, Error> {
        Ok(HandInsist {
            reason: got.reason()?,
            account: got.from_rescued_step(&ACCOUNT)?,
            contributor: got.contributor()?,
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

fn refreshing(outcome: &str) -> Journey {
    run_created(Refreshing::descriptor(), |recorder| {
        Refreshing::builder()
            .initial_data(ending(outcome))
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

fn insisting(outcome: &str) -> Journey {
    run_created(Insisting::descriptor(), |recorder| {
        Insisting::builder()
            .initial_data(ending(outcome))
            .reporter(recorder)
            .create()
            .unwrap()
    })
}

fn by_the_builder(rescue: impl RescueFactory, outcome: &str) -> Journey {
    let workflow: WorkflowDescriptor<Orders> = orders()
        .step(Charge::descriptor(step_name!("charge")).rescue(rescue))
        .build()
        .unwrap();
    run_created(&workflow, |recorder| {
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
#[case::busy("busy")]
#[case::crashed("abnormal")]
fn a_rescue_step_the_macros_describe_lists_and_runs_as_one_written_for_the_builder(
    #[case] outcome: &str,
) {
    assert_eq!(refreshing(outcome), by_the_builder(RefreshFactory, outcome));
}

#[rstest]
#[case::declined("failure")]
#[case::crashed("abnormal")]
fn a_rescue_steps_required_requests_are_resolved_as_the_builders_are(#[case] outcome: &str) {
    assert_eq!(insisting(outcome), by_the_builder(InsistFactory, outcome));
}
