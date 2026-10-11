//! What a required request found missing, as events tell it.

use crate::event::{FailureRequester, Requester};
use crate::step::StepName;

/// What a required request found missing: data under a key, or the failure's reason or the error
/// that a step hook requested from the step it acts on, or a rescue step from the step it
/// rescues.
///
/// # Examples
///
/// ```
/// use itinera::event::MissingData;
///
/// fn key(missing: &MissingData) -> Option<&str> {
///     match missing {
///         MissingData::Key { key, .. } => Some(key),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MissingData {
    /// Data under a key.
    #[non_exhaustive]
    Key {
        /// The key that was requested.
        key: String,
        /// Who requested it, and for which step.
        requester: Requester,
    },
    /// The failure's reason, which the attempt did not report.
    #[non_exhaustive]
    Reason {
        /// Who requested it, and for which step.
        requester: FailureRequester,
    },
    /// The error, which did not end the attempt.
    #[non_exhaustive]
    Error {
        /// Who requested it, and for which step.
        requester: FailureRequester,
    },
}

impl MissingData {
    pub(super) fn step(&self) -> Option<StepName> {
        match self {
            Self::Key { requester, .. } => requester.step(),
            Self::Reason { requester } | Self::Error { requester } => Some(requester.step()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{PolicyName, StepHook};

    #[test]
    fn a_required_request_for_a_missing_reason_names_the_step_the_hook_acts_on() {
        let missing = MissingData::Reason {
            requester: FailureRequester::StepHook {
                policy: PolicyName::from("alarm"),
                hook: StepHook::OnStepFailure,
                step: StepName::new("charge"),
            },
        };
        assert_eq!(missing.step(), Some(StepName::new("charge")));
    }

    #[test]
    fn a_rescue_steps_required_request_for_a_missing_error_names_the_rescue_step() {
        let missing = MissingData::Error {
            requester: FailureRequester::RescueStep {
                step: StepName::new("charge_rescue"),
            },
        };
        assert_eq!(missing.step(), Some(StepName::new("charge_rescue")));
    }
}
