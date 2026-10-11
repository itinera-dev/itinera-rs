//! How to call each hook a policy defines.

use super::Call;
use crate::policy::{
    FailWorkflow, OnSuccess, StepAbnormalTermination, StepFailure, StepRetry, StepSuccess,
    WorkflowFailure, WorkflowSuccess,
};

/// How to call each step hook a policy defines.
pub(super) struct StepCalls<P, W, M> {
    pub(super) success: Option<Call<P, W, StepSuccess, M, Option<OnSuccess>>>,
    pub(super) failure: Option<Call<P, W, StepFailure, M, Option<FailWorkflow>>>,
    pub(super) retry: Option<Call<P, W, StepRetry, M, Option<FailWorkflow>>>,
    pub(super) abnormal_termination:
        Option<Call<P, W, StepAbnormalTermination, M, Option<FailWorkflow>>>,
}

impl<P, W, M> Clone for StepCalls<P, W, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, W, M> Copy for StepCalls<P, W, M> {}

/// How to call each workflow hook a policy defines.
pub(super) struct WorkflowCalls<P, W, M> {
    pub(super) success: Option<Call<P, W, WorkflowSuccess, M, ()>>,
    pub(super) failure: Option<Call<P, W, WorkflowFailure, M, ()>>,
}

impl<P, W, M> Clone for WorkflowCalls<P, W, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, W, M> Copy for WorkflowCalls<P, W, M> {}
