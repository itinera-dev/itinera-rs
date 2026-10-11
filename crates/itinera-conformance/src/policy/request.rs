//! What a hook requests, as the scripts take it.

use crate::model::HookRequest;
use crate::value::Data;

/// What a hook requests, with its keys as itinera takes them. A role is not requested: a hook
/// reaches it when it calls it.
///
/// It displays as an error names it.
#[derive(Debug, derive_more::Display)]
pub(crate) enum Request {
    #[display("step data")]
    StepData(Data),
    #[display("data from the workflow")]
    WorkflowData(Data),
    #[display("the step name")]
    StepName,
    #[display("the attempt number")]
    Attempt,
    #[display("the failure cause")]
    FailureCause,
    #[display("the retry cause")]
    RetryCause,
    /// The failure reason, optional or required.
    #[display("the failure reason")]
    Reason { optional: bool },
    #[display("the error")]
    Error,
    #[display("the journey ID")]
    JourneyId,
}

impl Request {
    pub(super) fn of(request: &HookRequest) -> Option<Self> {
        Some(match request {
            HookRequest::StepData {
                key,
                value_type,
                optional,
            } => Self::StepData(Data::of(key, *value_type, *optional)),
            HookRequest::DataFromWorkflow {
                key,
                value_type,
                optional,
            } => Self::WorkflowData(Data::of(key, *value_type, *optional)),
            HookRequest::StepName => Self::StepName,
            HookRequest::AttemptNumber => Self::Attempt,
            HookRequest::FailureCause => Self::FailureCause,
            HookRequest::RetryCause => Self::RetryCause,
            HookRequest::FailureReason { optional } => Self::Reason {
                optional: *optional,
            },
            HookRequest::Error => Self::Error,
            HookRequest::JourneyId => Self::JourneyId,
            HookRequest::Role(_) => return None,
        })
    }
}
