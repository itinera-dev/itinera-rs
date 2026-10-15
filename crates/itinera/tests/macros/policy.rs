//! Policies the macros describe, against the same policies written for the builder, in a
//! synchronous workflow.

use itinera::error::Error;
use itinera::journey::{Contributor, JourneyId};
use itinera::policy::{
    HookNeeds, HookReporter, OnFailure, OnStepAbnormalTermination, OnStepFailure, OnStepRetry,
    OnStepSuccess, OnSuccess, OnWorkflowFailure, OnWorkflowSuccess, Requested, RetryCause,
    StepAbnormalTermination, StepFailure, StepFailureCause, StepHook, StepPolicy,
    StepPolicyDescriptor, StepRetry, StepSuccess, WorkflowFailure, WorkflowHook, WorkflowPolicy,
    WorkflowPolicyDescriptor, WorkflowSuccess, step_policy, workflow_policy,
};
use itinera::step::{
    Attempt, DescribedStep, Input, OptionalInput, Outcome, Reason, StepDescriptor, StepName, step,
    step_name,
};
use itinera::workflow::WorkflowDescriptor;
use rstest::rstest;

use super::fixtures::{
    CRASHED, CRASHED_THEN_REFUSED, DECLINED, DECLINED_UNTIL_RETRIES_RAN_OUT, Desk, Order, Orders,
    SETTLED, SETTLED_QUIETLY, SETTLED_THEN_BROKEN, SETTLED_THEN_FINISHED, SETTLED_THEN_REFUSED,
    orders, run,
};

/// A step that ends as its initial data says.
struct Settle {
    outcome: String,
}

#[step]
impl Settle {
    fn new(#[input] outcome: String) -> Self {
        Self { outcome }
    }

    fn run(self, contributor: &mut Contributor) -> Result<Outcome, Error> {
        match self.outcome.as_str() {
            "success" => {
                contributor.contribute("receipt", "R-1".to_owned());
                Self::succeed()
            }
            "failure" => Self::fail(Reason::new("declined").with_message("the card was declined")),
            "busy" => Self::fail("busy"),
            "quiet" => Self::succeed(),
            _ => Err(Error::msg("the till crashed")),
        }
    }
}

#[derive(Default)]
struct Audit;

#[step_policy(name = "audit")]
impl Audit {
    #[on_step_success]
    fn succeeded(
        &self,
        #[from_step] receipt: String,
        #[from_workflow] decision: Option<String>,
        step: StepName,
        attempt: Attempt,
        mut reporter: HookReporter<'_>,
    ) -> Result<OnSuccess, Error> {
        reporter.info(format!("{step} attempt {attempt} gave {receipt}"))?;
        match decision.as_deref() {
            Some("finish") => Self::finish_workflow(),
            Some("fail") => Self::fail_workflow("refused"),
            Some("break") => Err(Error::msg("the audit log is full")),
            _ => Self::keep_default(),
        }
    }

    #[on_step_failure]
    fn failed(
        &self,
        reason: Option<Reason>,
        cause: StepFailureCause,
        error: Option<&Error>,
        contributor: &mut Contributor,
    ) -> Result<OnFailure, Error> {
        let code = reason.as_ref().map(Reason::code);
        let error = error.map(ToString::to_string);
        contributor.contribute("failure", format!("{cause}: {code:?} {error:?}"));
        Self::keep_default()
    }

    #[on_step_retry]
    fn retried(
        &self,
        cause: RetryCause,
        reason: Reason,
        journey_id: &JourneyId,
        #[role] desk: &dyn Desk,
        mut reporter: HookReporter<'_>,
    ) -> Result<OnFailure, Error> {
        let code = reason.code();
        let desk = desk.desk();
        reporter.info(format!(
            "{journey_id} retried after {cause} {code} at the {desk}"
        ))?;
        Self::keep_default()
    }

    #[on_step_abnormal_termination]
    fn terminated(
        &self,
        error: &Error,
        #[from_workflow("decision")] decision: Option<String>,
    ) -> Result<OnFailure, Error> {
        match decision.as_deref() {
            Some("fail") => {
                Self::fail_workflow(Reason::new("crashed").with_message(error.to_string()))
            }
            _ => Self::keep_default(),
        }
    }
}

struct Notify {
    channel: &'static str,
}

#[workflow_policy]
impl Notify {
    #[factory]
    fn opened() -> Self {
        Self { channel: "orders" }
    }

    #[on_workflow_success]
    fn succeeded(
        &self,
        #[from_workflow] receipt: Option<String>,
        contributor: &mut Contributor,
    ) -> Result<(), Error> {
        contributor.contribute("notified", format!("{} {receipt:?}", self.channel));
        Ok(())
    }

    #[on_workflow_failure]
    fn failed(&self, #[role] desk: &dyn Desk, mut reporter: HookReporter<'_>) -> Result<(), Error> {
        reporter.warning(format!("{} told the {}", self.channel, desk.desk()))?;
        Ok(())
    }
}

const RECEIPT: Input<String> = Input::new("receipt");
const DECISION: OptionalInput<String> = OptionalInput::new("decision");
const RECEIPT_IF_ANY: OptionalInput<String> = OptionalInput::new("receipt");

struct HandAudit;

impl OnStepSuccess<Orders> for HandAudit {
    fn needs() -> HookNeeds<StepSuccess> {
        HookNeeds::new()
            .from_step(&RECEIPT)
            .optional_from_workflow(&DECISION)
            .reporter()
    }

    fn on_step_success(
        &self,
        mut got: Requested<'_, Orders, StepSuccess>,
    ) -> Result<OnSuccess, Error> {
        let receipt = got.from_step(&RECEIPT)?;
        let decision = got.optional_from_workflow(&DECISION)?;
        let (step, attempt) = (got.step_name(), got.attempt());
        got.reporter()?
            .info(format!("{step} attempt {attempt} gave {receipt}"))?;
        Ok(match decision.as_deref() {
            Some("finish") => OnSuccess::FinishWorkflow,
            Some("fail") => OnSuccess::fail_workflow("refused"),
            Some("break") => return Err(Error::msg("the audit log is full")),
            _ => OnSuccess::KeepDefault,
        })
    }
}

impl OnStepFailure<Orders> for HandAudit {
    fn needs() -> HookNeeds<StepFailure> {
        HookNeeds::new()
            .optional_reason()
            .optional_error()
            .contributor()
    }

    fn on_step_failure(
        &self,
        mut got: Requested<'_, Orders, StepFailure>,
    ) -> Result<OnFailure, Error> {
        let reason = got.optional_reason()?;
        let cause = got.cause();
        let error = got.optional_error()?.map(ToString::to_string);
        let code = reason.as_ref().map(Reason::code);
        got.contributor()?
            .contribute("failure", format!("{cause}: {code:?} {error:?}"));
        Ok(OnFailure::KeepDefault)
    }
}

impl OnStepRetry<Orders> for HandAudit {
    fn needs() -> HookNeeds<StepRetry> {
        HookNeeds::new().reason().reporter()
    }

    fn on_step_retry(&self, mut got: Requested<'_, Orders, StepRetry>) -> Result<OnFailure, Error> {
        let code = got.reason()?.code().to_owned();
        let desk = got.role::<dyn Desk>().desk();
        let (cause, journey_id) = (got.cause(), got.journey_id());
        got.reporter()?.info(format!(
            "{journey_id} retried after {cause} {code} at the {desk}"
        ))?;
        Ok(OnFailure::KeepDefault)
    }
}

impl OnStepAbnormalTermination<Orders> for HandAudit {
    fn needs() -> HookNeeds<StepAbnormalTermination> {
        HookNeeds::new().error().optional_from_workflow(&DECISION)
    }

    fn on_step_abnormal_termination(
        &self,
        mut got: Requested<'_, Orders, StepAbnormalTermination>,
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

struct HandNotify {
    channel: &'static str,
}

impl OnWorkflowSuccess<Orders> for HandNotify {
    fn needs() -> HookNeeds<WorkflowSuccess> {
        HookNeeds::new()
            .optional_from_workflow(&RECEIPT_IF_ANY)
            .contributor()
    }

    fn on_workflow_success(
        &self,
        mut got: Requested<'_, Orders, WorkflowSuccess>,
    ) -> Result<(), Error> {
        let receipt = got.optional_from_workflow(&RECEIPT_IF_ANY)?;
        got.contributor()?
            .contribute("notified", format!("{} {receipt:?}", self.channel));
        Ok(())
    }
}

impl OnWorkflowFailure<Orders> for HandNotify {
    fn needs() -> HookNeeds<WorkflowFailure> {
        HookNeeds::new().reporter()
    }

    fn on_workflow_failure(
        &self,
        mut got: Requested<'_, Orders, WorkflowFailure>,
    ) -> Result<(), Error> {
        let desk = got.role::<dyn Desk>().desk();
        got.reporter()?
            .warning(format!("{} told the {desk}", self.channel))?;
        Ok(())
    }
}

fn settle() -> StepDescriptor<Orders> {
    Settle::descriptor(step_name!("settle")).retry_budget(1)
}

fn by_macros() -> WorkflowDescriptor<Orders> {
    orders()
        .step(settle().policy(Audit::descriptor()))
        .policy(Notify::descriptor())
        .build()
        .unwrap()
}

fn opened_by_hand() -> Result<HandNotify, Error> {
    Ok(HandNotify { channel: "orders" })
}

fn by_hand() -> WorkflowDescriptor<Orders> {
    let audit = StepPolicyDescriptor::new("audit", || HandAudit)
        .on_step_success()
        .on_step_failure()
        .on_step_retry()
        .on_step_abnormal_termination();
    let notify = WorkflowPolicyDescriptor::fallible("Notify", opened_by_hand)
        .on_workflow_success()
        .on_workflow_failure();
    orders()
        .step(settle().policy(audit))
        .policy(notify)
        .build()
        .unwrap()
}

#[rstest]
#[case::settled(SETTLED)]
#[case::settled_then_finished(SETTLED_THEN_FINISHED)]
#[case::settled_then_refused(SETTLED_THEN_REFUSED)]
#[case::settled_then_broken(SETTLED_THEN_BROKEN)]
#[case::settled_without_a_receipt(SETTLED_QUIETLY)]
#[case::declined(DECLINED)]
#[case::declined_until_retries_ran_out(DECLINED_UNTIL_RETRIES_RAN_OUT)]
#[case::crashed(CRASHED)]
#[case::crashed_then_refused(CRASHED_THEN_REFUSED)]
fn policies_the_macros_describe_list_and_run_as_those_written_for_the_builder(
    #[case] order: Order,
) {
    assert_eq!(run(&by_macros(), order), run(&by_hand(), order));
}

#[test]
fn a_step_policy_lists_the_hooks_it_defines_in_the_order_its_descriptor_defines_them() {
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
fn a_workflow_policy_lists_the_hooks_it_defines_in_the_order_its_descriptor_defines_them() {
    assert_eq!(
        <Notify as WorkflowPolicy<Orders>>::HOOKS,
        [
            WorkflowHook::OnWorkflowSuccess,
            WorkflowHook::OnWorkflowFailure
        ]
    );
}

struct Closed;

#[step_policy]
impl Closed {
    #[factory]
    fn opened() -> Result<Self, Error> {
        Err(Error::msg("the audit log is closed"))
    }

    #[on_step_success]
    fn succeeded(&self) -> Result<OnSuccess, Error> {
        Self::keep_default()
    }
}

struct HandClosed;

impl OnStepSuccess<Orders> for HandClosed {
    fn needs() -> HookNeeds<StepSuccess> {
        HookNeeds::new()
    }

    fn on_step_success(
        &self,
        _got: Requested<'_, Orders, StepSuccess>,
    ) -> Result<OnSuccess, Error> {
        Ok(OnSuccess::KeepDefault)
    }
}

fn closed_by_hand() -> Result<HandClosed, Error> {
    Err(Error::msg("the audit log is closed"))
}

#[test]
fn a_policy_whose_factory_fails_runs_as_one_written_for_the_builder() {
    let by_macros = orders()
        .step(settle().policy(Closed::descriptor()))
        .build()
        .unwrap();
    let closed = StepPolicyDescriptor::fallible("Closed", closed_by_hand).on_step_success();
    let by_hand = orders().step(settle().policy(closed)).build().unwrap();
    assert_eq!(run(&by_macros, SETTLED), run(&by_hand, SETTLED));
}

/// A policy named like the parameter the generated code would otherwise give the workflow.
#[derive(Default)]
struct W;

#[step_policy]
impl W {
    #[on_step_success]
    fn succeeded(&self, #[role] desk: &dyn Desk) -> Result<OnSuccess, Error> {
        match desk.desk() {
            "orders desk" => Self::keep_default(),
            _ => Self::fail_workflow("lost"),
        }
    }
}

#[test]
fn a_policy_may_take_any_name_the_generated_code_uses() {
    let workflow = orders().step(settle().policy(W::descriptor())).build();
    assert!(workflow.is_ok());
}
