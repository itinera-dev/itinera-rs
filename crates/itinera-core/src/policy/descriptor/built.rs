//! Policies built from their descriptors, for one attempt or one journey, whose hooks the
//! executor calls.

use std::future::ready;

use super::HookCall;
use super::calls::{StepCalls, WorkflowCalls};
use crate::policy::{
    FailWorkflow, OnSuccess, Requested, StepAbnormalTermination, StepFailure, StepRetry,
    StepSuccess, WorkflowFailure, WorkflowSuccess,
};

/// An instance of a step policy, built for one attempt, whose hooks the executor calls.
pub(crate) trait BuiltStepPolicy<W, M>: Send + Sync {
    fn on_step_success<'a>(
        &'a self,
        got: Requested<'a, W, StepSuccess, M>,
    ) -> HookCall<'a, Option<OnSuccess>>;

    fn on_step_failure<'a>(
        &'a self,
        got: Requested<'a, W, StepFailure, M>,
    ) -> HookCall<'a, Option<FailWorkflow>>;

    fn on_step_retry<'a>(
        &'a self,
        got: Requested<'a, W, StepRetry, M>,
    ) -> HookCall<'a, Option<FailWorkflow>>;

    fn on_step_abnormal_termination<'a>(
        &'a self,
        got: Requested<'a, W, StepAbnormalTermination, M>,
    ) -> HookCall<'a, Option<FailWorkflow>>;
}

/// A built policy, with how to call each hook it defines.
pub(super) struct Built<P, C> {
    pub(super) policy: P,
    pub(super) calls: C,
}

impl<P, W, M> BuiltStepPolicy<W, M> for Built<P, StepCalls<P, W, M>>
where
    P: Send + Sync + 'static,
{
    fn on_step_success<'a>(
        &'a self,
        got: Requested<'a, W, StepSuccess, M>,
    ) -> HookCall<'a, Option<OnSuccess>> {
        match self.calls.success {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }

    fn on_step_failure<'a>(
        &'a self,
        got: Requested<'a, W, StepFailure, M>,
    ) -> HookCall<'a, Option<FailWorkflow>> {
        match self.calls.failure {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }

    fn on_step_retry<'a>(
        &'a self,
        got: Requested<'a, W, StepRetry, M>,
    ) -> HookCall<'a, Option<FailWorkflow>> {
        match self.calls.retry {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }

    fn on_step_abnormal_termination<'a>(
        &'a self,
        got: Requested<'a, W, StepAbnormalTermination, M>,
    ) -> HookCall<'a, Option<FailWorkflow>> {
        match self.calls.abnormal_termination {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }
}

/// What a hook the policy does not define answers: nothing, the default.
fn nothing<'a, T: Default + Send + 'a>() -> HookCall<'a, T> {
    Box::pin(ready(Ok(T::default())))
}

/// An instance of a workflow policy, built for one journey, whose hooks the executor calls.
pub(crate) trait BuiltWorkflowPolicy<W, M>: Send + Sync {
    fn on_workflow_success<'a>(
        &'a self,
        got: Requested<'a, W, WorkflowSuccess, M>,
    ) -> HookCall<'a, ()>;

    fn on_workflow_failure<'a>(
        &'a self,
        got: Requested<'a, W, WorkflowFailure, M>,
    ) -> HookCall<'a, ()>;
}

impl<P, W, M> BuiltWorkflowPolicy<W, M> for Built<P, WorkflowCalls<P, W, M>>
where
    P: Send + Sync + 'static,
{
    fn on_workflow_success<'a>(
        &'a self,
        got: Requested<'a, W, WorkflowSuccess, M>,
    ) -> HookCall<'a, ()> {
        match self.calls.success {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }

    fn on_workflow_failure<'a>(
        &'a self,
        got: Requested<'a, W, WorkflowFailure, M>,
    ) -> HookCall<'a, ()> {
        match self.calls.failure {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }
}
