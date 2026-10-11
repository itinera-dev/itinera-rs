//! How a journey ends before its last step, instead of succeeding: failed or aborted, for the
//! result and as events report it.

use crate::event::{JourneyAbort, JourneyFailure};
use crate::journey::{Abort, Failure};
use crate::step::StepName;

/// How a journey ends before its last step, instead of succeeding.
#[derive(derive_more::From)]
pub(super) enum End {
    Failed(Box<Failing>),
    Aborted(Box<Aborted>),
}

/// A failure of the journey, for the result and as `journey_failed` reports it.
pub(super) struct Failing {
    /// The step that failed, or whose hook returned `FailWorkflow`.
    pub(super) step: StepName,
    pub(super) reported: JourneyFailure,
    pub(super) failure: Failure,
}

/// An abort, for the result and as `journey_aborted` reports it.
pub(super) struct Aborted {
    pub(super) abort: Abort,
    pub(super) reported: JourneyAbort,
}

impl End {
    pub(super) fn failed(step: StepName, reported: JourneyFailure, failure: Failure) -> Self {
        Self::Failed(Box::new(Failing {
            step,
            reported,
            failure,
        }))
    }

    pub(super) fn aborted(abort: Abort, reported: JourneyAbort) -> Self {
        Self::Aborted(Box::new(Aborted { abort, reported }))
    }
}
