//! The points where the engine calls a step's hooks after an attempt, and what they may answer.

use crate::event::GiveUpHook;
use crate::policy::PolicyName;
use crate::step::{Reason, StepAttempt};

/// A lifecycle a policy's hook returned, which decides what happens next.
#[derive(Debug)]
pub(crate) struct Decided<L> {
    pub(crate) policy: PolicyName,
    pub(crate) lifecycle: L,
}

/// What `on step success` may return.
#[derive(Debug)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "no hook returns a lifecycle until hooks can be written"
    )
)]
pub(crate) enum OnSuccess {
    FinishWorkflow,
    FailWorkflow(Reason),
}

/// The step hooks the engine calls after each attempt: `on step success` after a success;
/// `on step abnormal termination` before the step's own rule decides what follows an abnormal
/// termination; `on step retry` once a retry is decided, before `step_retrying`; and
/// `on step failure` after `step_given_up`. Each answers with the lifecycle a policy's hook
/// returned, or nothing, which keeps the default.
pub(crate) trait StepHooks: Send {
    /// `on step success`, after an attempt that succeeded.
    fn on_step_success(&mut self, attempt: &StepAttempt) -> Option<Decided<OnSuccess>>;

    /// `on step retry` or `on step abnormal termination`, which may give the step up with
    /// `FailWorkflow` and its reason.
    fn may_give_up(&mut self, hook: GiveUpHook, attempt: &StepAttempt) -> Option<Decided<Reason>>;

    /// `on step failure`, after the step was given up, which may fail the journey with a reason
    /// of its own.
    fn on_step_failure(&mut self, attempt: &StepAttempt) -> Option<Decided<Reason>>;
}

/// Step hooks that keep their default behaviour, as every hook does until hooks can be written.
pub(crate) struct Defaults;

impl StepHooks for Defaults {
    fn on_step_success(&mut self, _: &StepAttempt) -> Option<Decided<OnSuccess>> {
        None
    }

    fn may_give_up(&mut self, _: GiveUpHook, _: &StepAttempt) -> Option<Decided<Reason>> {
        None
    }

    fn on_step_failure(&mut self, _: &StepAttempt) -> Option<Decided<Reason>> {
        None
    }
}
