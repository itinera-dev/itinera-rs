//! Asynchronous policies the macros describe, against the same policies written for the builder,
//! in an asynchronous workflow.

use itinera::error::Error;
use itinera::journey::{Contributor, JourneyId};
use itinera::mode::Asynchronous;
use itinera::policy::{
    AsyncHookReporter, AsyncOnStepAbnormalTermination, AsyncOnStepFailure, AsyncOnStepRetry,
    AsyncOnStepSuccess, AsyncOnWorkflowFailure, AsyncOnWorkflowSuccess, HookNeeds, OnFailure,
    OnSuccess, Requested, RetryCause, StepAbnormalTermination, StepFailure, StepHook, StepPolicy,
    StepPolicyDescriptor, StepRetry, StepSuccess, WorkflowFailure, WorkflowHook, WorkflowPolicy,
    WorkflowPolicyDescriptor, WorkflowSuccess, step_policy, workflow_policy,
};
use itinera::step::{
    DescribedStep, Input, OptionalInput, Outcome, Reason, StepDescriptor, step, step_name,
};
use itinera::workflow::WorkflowDescriptor;
use rstest::rstest;

use super::fixtures::{
    CRASHED, CRASHED_THEN_REFUSED, DECLINED, DECLINED_UNTIL_RETRIES_RAN_OUT, Desk, Order, Orders,
    SETTLED, SETTLED_THEN_FINISHED, SETTLED_THEN_REFUSED, async_orders, run_async,
};

struct Settle {
    outcome: String,
}

#[step]
impl Settle {
    fn new(#[input] outcome: String) -> Self {
        Self { outcome }
    }

    async fn run(self, #[contributor] contributor: &mut Contributor) -> Result<Outcome, Error> {
        match self.outcome.as_str() {
            "success" => {
                contributor.contribute("receipt", "R-1".to_owned());
                Self::succeed()
            }
            "failure" => Self::fail("declined"),
            "busy" => Self::fail("busy"),
            _ => Err(Error::msg("the till crashed")),
        }
    }
}

#[derive(Default)]
struct Audit;

#[step_policy(name = "audit")]
impl Audit {
    #[on_step_success]
    async fn succeeded(
        &self,
        #[from_step] receipt: String,
        #[from_workflow] decision: Option<String>,
        #[reporter] mut reporter: AsyncHookReporter<'_>,
    ) -> Result<OnSuccess, Error> {
        reporter.info(format!("gave {receipt}")).await?;
        match decision.as_deref() {
            Some("finish") => Self::finish_workflow(),
            Some("fail") => Self::fail_workflow("refused"),
            _ => Self::keep_default(),
        }
    }

    #[on_step_failure]
    async fn failed(&self, #[reason] reason: Reason) -> Result<OnFailure, Error> {
        Self::fail_workflow(format!("after-{}", reason.code()))
    }

    #[on_step_retry]
    async fn retried(
        &self,
        #[cause] cause: RetryCause,
        #[journey_id] journey_id: &JourneyId,
        #[role] desk: &dyn Desk,
        #[reporter] mut reporter: AsyncHookReporter<'_>,
    ) -> Result<OnFailure, Error> {
        let desk = desk.desk();
        reporter
            .info(format!("{journey_id} retried after {cause} at the {desk}"))
            .await?;
        Self::keep_default()
    }

    #[on_step_abnormal_termination]
    async fn terminated(
        &self,
        #[error] error: &Error,
        #[from_workflow] decision: Option<String>,
    ) -> Result<OnFailure, Error> {
        match decision.as_deref() {
            Some("fail") => {
                Self::fail_workflow(Reason::new("crashed").with_message(error.to_string()))
            }
            _ => Self::keep_default(),
        }
    }
}

#[derive(Default)]
struct Notify;

#[workflow_policy(name = "notify")]
impl Notify {
    #[on_workflow_success]
    async fn succeeded(&self, #[contributor] contributor: &mut Contributor) -> Result<(), Error> {
        contributor.contribute("notified", true);
        Ok(())
    }

    #[on_workflow_failure]
    async fn failed(&self, #[reporter] mut reporter: AsyncHookReporter<'_>) -> Result<(), Error> {
        reporter.warning("the order failed").await?;
        Ok(())
    }
}

const RECEIPT: Input<String> = Input::new("receipt");
const DECISION: OptionalInput<String> = OptionalInput::new("decision");

struct HandAudit;

impl AsyncOnStepSuccess<Orders> for HandAudit {
    fn needs() -> HookNeeds<StepSuccess> {
        HookNeeds::new()
            .from_step(&RECEIPT)
            .optional_from_workflow(&DECISION)
            .reporter()
    }

    async fn on_step_success(
        &self,
        mut got: Requested<'_, Orders, StepSuccess, Asynchronous>,
    ) -> Result<OnSuccess, Error> {
        let receipt = got.from_step(&RECEIPT)?;
        let decision = got.optional_from_workflow(&DECISION)?;
        got.reporter()?.info(format!("gave {receipt}")).await?;
        Ok(match decision.as_deref() {
            Some("finish") => OnSuccess::FinishWorkflow,
            Some("fail") => OnSuccess::fail_workflow("refused"),
            _ => OnSuccess::KeepDefault,
        })
    }
}

impl AsyncOnStepFailure<Orders> for HandAudit {
    fn needs() -> HookNeeds<StepFailure> {
        HookNeeds::new().reason()
    }

    async fn on_step_failure(
        &self,
        mut got: Requested<'_, Orders, StepFailure, Asynchronous>,
    ) -> Result<OnFailure, Error> {
        let code = format!("after-{}", got.reason()?.code());
        Ok(OnFailure::fail_workflow(code))
    }
}

impl AsyncOnStepRetry<Orders> for HandAudit {
    fn needs() -> HookNeeds<StepRetry> {
        HookNeeds::new().reporter()
    }

    async fn on_step_retry(
        &self,
        mut got: Requested<'_, Orders, StepRetry, Asynchronous>,
    ) -> Result<OnFailure, Error> {
        let desk = got.role::<dyn Desk>().desk();
        let (cause, journey_id) = (got.cause(), got.journey_id());
        got.reporter()?
            .info(format!("{journey_id} retried after {cause} at the {desk}"))
            .await?;
        Ok(OnFailure::KeepDefault)
    }
}

impl AsyncOnStepAbnormalTermination<Orders> for HandAudit {
    fn needs() -> HookNeeds<StepAbnormalTermination> {
        HookNeeds::new().error().optional_from_workflow(&DECISION)
    }

    async fn on_step_abnormal_termination(
        &self,
        mut got: Requested<'_, Orders, StepAbnormalTermination, Asynchronous>,
    ) -> Result<OnFailure, Error> {
        let error = got.error()?;
        Ok(match got.optional_from_workflow(&DECISION)?.as_deref() {
            Some("fail") => {
                OnFailure::fail_workflow(Reason::new("crashed").with_message(error.to_string()))
            }
            _ => OnFailure::KeepDefault,
        })
    }
}

struct HandNotify;

impl AsyncOnWorkflowSuccess<Orders> for HandNotify {
    fn needs() -> HookNeeds<WorkflowSuccess> {
        HookNeeds::new().contributor()
    }

    async fn on_workflow_success(
        &self,
        mut got: Requested<'_, Orders, WorkflowSuccess, Asynchronous>,
    ) -> Result<(), Error> {
        got.contributor()?.contribute("notified", true);
        Ok(())
    }
}

impl AsyncOnWorkflowFailure<Orders> for HandNotify {
    fn needs() -> HookNeeds<WorkflowFailure> {
        HookNeeds::new().reporter()
    }

    async fn on_workflow_failure(
        &self,
        mut got: Requested<'_, Orders, WorkflowFailure, Asynchronous>,
    ) -> Result<(), Error> {
        got.reporter()?.warning("the order failed").await?;
        Ok(())
    }
}

fn settle() -> StepDescriptor<Orders, Asynchronous> {
    Settle::descriptor(step_name!("settle")).retry_budget(1)
}

fn by_macros() -> WorkflowDescriptor<Orders, Asynchronous> {
    async_orders()
        .step(settle().policy(Audit::descriptor()))
        .policy(Notify::descriptor())
        .build()
        .unwrap()
}

fn by_hand() -> WorkflowDescriptor<Orders, Asynchronous> {
    let audit = StepPolicyDescriptor::new_async("audit", || HandAudit)
        .on_step_success()
        .on_step_failure()
        .on_step_retry()
        .on_step_abnormal_termination();
    let notify = WorkflowPolicyDescriptor::new_async("notify", || HandNotify)
        .on_workflow_success()
        .on_workflow_failure();
    async_orders()
        .step(settle().policy(audit))
        .policy(notify)
        .build()
        .unwrap()
}

#[rstest]
#[case::settled(SETTLED)]
#[case::settled_then_finished(SETTLED_THEN_FINISHED)]
#[case::settled_then_refused(SETTLED_THEN_REFUSED)]
#[case::declined(DECLINED)]
#[case::declined_until_retries_ran_out(DECLINED_UNTIL_RETRIES_RAN_OUT)]
#[case::crashed(CRASHED)]
#[case::crashed_then_refused(CRASHED_THEN_REFUSED)]
fn asynchronous_policies_the_macros_describe_list_and_run_as_those_written_for_the_builder(
    #[case] order: Order,
) {
    assert_eq!(run_async(&by_macros(), order), run_async(&by_hand(), order));
}

#[test]
fn an_asynchronous_step_policy_lists_the_hooks_it_defines() {
    assert_eq!(
        <Audit as StepPolicy<Orders>>::HOOKS,
        [
            StepHook::OnStepSuccess,
            StepHook::OnStepFailure,
            StepHook::OnStepRetry,
            StepHook::OnStepAbnormalTermination,
        ]
    );
}

#[test]
fn an_asynchronous_workflow_policy_lists_the_hooks_it_defines() {
    assert_eq!(
        <Notify as WorkflowPolicy<Orders>>::HOOKS,
        [
            WorkflowHook::OnWorkflowSuccess,
            WorkflowHook::OnWorkflowFailure
        ]
    );
}
