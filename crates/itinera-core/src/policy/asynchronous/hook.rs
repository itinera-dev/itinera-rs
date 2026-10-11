//! The asynchronous hooks a policy may define, one trait each.

mod on_step_abnormal_termination;
mod on_step_failure;
mod on_step_retry;
mod on_step_success;
mod on_workflow_failure;
mod on_workflow_success;

pub use on_step_abnormal_termination::AsyncOnStepAbnormalTermination;
pub use on_step_failure::AsyncOnStepFailure;
pub use on_step_retry::AsyncOnStepRetry;
pub use on_step_success::AsyncOnStepSuccess;
pub use on_workflow_failure::AsyncOnWorkflowFailure;
pub use on_workflow_success::AsyncOnWorkflowSuccess;
