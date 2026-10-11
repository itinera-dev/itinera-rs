//! The synchronous hooks a policy may define, one trait each.

mod on_step_abnormal_termination;
mod on_step_failure;
mod on_step_retry;
mod on_step_success;
mod on_workflow_failure;
mod on_workflow_success;

pub use on_step_abnormal_termination::OnStepAbnormalTermination;
pub use on_step_failure::OnStepFailure;
pub use on_step_retry::OnStepRetry;
pub use on_step_success::OnStepSuccess;
pub use on_workflow_failure::OnWorkflowFailure;
pub use on_workflow_success::OnWorkflowSuccess;
