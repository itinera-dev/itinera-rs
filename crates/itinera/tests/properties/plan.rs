//! Generated workflows: what each part of a workflow does in its one journey, as plain data, and
//! the strategies that generate it.

use std::num::NonZeroUsize;

use derive_more::From;
use proptest::bool::weighted;
use proptest::collection::vec;
use proptest::option;
use proptest::prelude::{Just, Strategy, any, prop_oneof};

/// A workflow and its journey: its steps in order, its workflow policy if it has one, and the
/// event on which a flaky reporter of the journey fails, if it fails at all.
#[derive(Debug, From)]
pub(crate) struct Plan {
    pub(crate) steps: Vec<StepPlan>,
    pub(crate) closing: Option<Closing>,
    pub(crate) reporter_fails_on: Option<NonZeroUsize>,
}

/// A step: its descriptor's settings, whether its policy is attached, and what each attempt it
/// may make does, one entry per attempt its retry budget allows.
#[derive(Debug, From)]
pub(crate) struct StepPlan {
    pub(crate) retry_budget: u16,
    pub(crate) retry_on_abnormal_termination: bool,
    pub(crate) has_policy: bool,
    pub(crate) attempts: Vec<Attempt>,
}

/// What one attempt of a step does, and what the step policy's hooks return for it.
#[derive(Clone, Copy, Debug, From)]
pub(crate) struct Attempt {
    pub(crate) ending: Ending,
    pub(crate) on_step_success: OnSuccessReturns,
    pub(crate) on_step_failure: Returns,
    pub(crate) on_step_retry: Returns,
    pub(crate) on_step_abnormal_termination: Returns,
}

/// How an attempt ends: one of the outcomes, or an abnormal termination.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Ending {
    Success,
    Failure,
    Skipped,
    AbnormalTermination,
}

/// What `on step success` returns.
#[derive(Clone, Copy, Debug)]
pub(crate) enum OnSuccessReturns {
    Nothing,
    FinishWorkflow,
    FailWorkflow,
    Error,
}

/// What `on step failure`, `on step retry` or `on step abnormal termination` returns.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Returns {
    Nothing,
    FailWorkflow,
    Error,
}

/// The workflow policy: whether each of its hooks fails.
#[derive(Clone, Copy, Debug, From)]
pub(crate) struct Closing {
    pub(crate) on_workflow_success_fails: bool,
    pub(crate) on_workflow_failure_fails: bool,
}

/// Workflows of one to three steps, each with a retry budget of up to three.
pub(crate) fn plans() -> impl Strategy<Value = Plan> {
    (
        vec(steps(), 1..=3),
        option::of(closings()),
        option::weighted(0.3, failing_events()),
    )
        .prop_map(Plan::from)
}

/// The event a flaky reporter fails on, among the first thirty of the journey.
fn failing_events() -> impl Strategy<Value = NonZeroUsize> {
    (1..=30_usize).prop_filter_map("events are counted from 1", NonZeroUsize::new)
}

fn steps() -> impl Strategy<Value = StepPlan> {
    (0..=3_u16, any::<bool>(), any::<bool>()).prop_flat_map(with_attempts)
}

fn with_attempts(
    (retry_budget, retry_on_abnormal_termination, has_policy): (u16, bool, bool),
) -> impl Strategy<Value = StepPlan> {
    (
        Just(retry_budget),
        Just(retry_on_abnormal_termination),
        Just(has_policy),
        vec(attempts(), usize::from(retry_budget) + 1),
    )
        .prop_map(StepPlan::from)
}

fn attempts() -> impl Strategy<Value = Attempt> {
    (
        endings(),
        on_success_returns(),
        returns(),
        returns(),
        returns(),
    )
        .prop_map(Attempt::from)
}

fn endings() -> impl Strategy<Value = Ending> {
    prop_oneof![
        3 => Just(Ending::Success),
        4 => Just(Ending::Failure),
        1 => Just(Ending::Skipped),
        2 => Just(Ending::AbnormalTermination),
    ]
}

fn on_success_returns() -> impl Strategy<Value = OnSuccessReturns> {
    prop_oneof![
        8 => Just(OnSuccessReturns::Nothing),
        1 => Just(OnSuccessReturns::FinishWorkflow),
        1 => Just(OnSuccessReturns::FailWorkflow),
        1 => Just(OnSuccessReturns::Error),
    ]
}

fn returns() -> impl Strategy<Value = Returns> {
    prop_oneof![
        8 => Just(Returns::Nothing),
        1 => Just(Returns::FailWorkflow),
        1 => Just(Returns::Error),
    ]
}

fn closings() -> impl Strategy<Value = Closing> {
    (weighted(0.1), weighted(0.1)).prop_map(Closing::from)
}
