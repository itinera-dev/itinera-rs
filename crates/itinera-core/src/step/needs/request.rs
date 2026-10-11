//! What a step requests, inputs or, for a rescue step, what the step it rescues left.

use super::{InputNeed, Requirement};

/// One thing a step requests, as the executor resolves it before building the step: an input, or
/// what a rescue step requests from the step it rescues.
#[derive(Clone, Debug)]
pub(crate) enum StepRequest {
    Input(InputNeed),
    Rescued(RescueRequest),
}

/// What a rescue step requests from the failed attempt of the step it rescues: data it
/// contributed, the failure's reason or the error.
#[derive(Clone, Debug)]
pub(crate) enum RescueRequest {
    FromRescuedStep(InputNeed),
    Reason(Requirement),
    Error(Requirement),
}

pub(super) fn other_than_reason(request: &StepRequest) -> bool {
    !matches!(request, StepRequest::Rescued(RescueRequest::Reason(_)))
}

pub(super) fn other_than_error(request: &StepRequest) -> bool {
    !matches!(request, StepRequest::Rescued(RescueRequest::Error(_)))
}
