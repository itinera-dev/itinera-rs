//! Why a journey was aborted, as events tell it.

use super::Requester;
use crate::journey::AbortReason;
use crate::policy::{PolicyName, StepHook};
use crate::step::StepName;

/// Why a journey was aborted, with exactly what that reason carries: the step during which it
/// happened, when there was one, its details, and the message of the error when failing custom
/// code caused it.
///
/// # Examples
///
/// ```
/// use itinera::event::JourneyAbort;
///
/// fn describe(abort: &JourneyAbort) -> String {
///     match abort.error() {
///         Some(error) => format!("{}: {error}", abort.reason()),
///         None => abort.reason().to_string(),
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum JourneyAbort {
    /// A step's constructor, or the input adapter resolving one of its inputs, failed.
    #[non_exhaustive]
    StepCouldNotBeBuilt {
        /// The step's name.
        step: StepName,
        /// The error's message.
        error: String,
    },
    /// A step policy failed while being built for an attempt.
    #[non_exhaustive]
    PolicyCouldNotBeBuilt {
        /// The step's name.
        step: StepName,
        /// The policy's name.
        policy: PolicyName,
        /// The error's message.
        error: String,
    },
    /// A required request has no value.
    #[non_exhaustive]
    RequiredDataMissing {
        /// What was requested, and by whom.
        missing: MissingData,
    },
    /// A requested value has the wrong type.
    #[non_exhaustive]
    WrongType {
        /// The key that was requested.
        key: String,
        /// Who requested it, and for which step.
        requester: Requester,
    },
    /// A hook, or a role operation it called, failed.
    #[non_exhaustive]
    HookFailed {
        /// The name of the step a step hook acts on; `None` for a workflow hook.
        step: Option<StepName>,
        /// The error's message.
        error: String,
    },
    /// A reporter, or a dispatcher while dispatching, failed.
    #[non_exhaustive]
    ReporterFailed {
        /// The name of the step during which it happened, if any.
        step: Option<StepName>,
        /// The error's message.
        error: String,
    },
}
impl JourneyAbort {
    /// The abort's reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::JourneyAbort;
    /// use itinera::journey::AbortReason;
    ///
    /// fn caused_by_a_reporter(abort: &JourneyAbort) -> bool {
    ///     abort.reason() == AbortReason::ReporterFailed
    /// }
    /// ```
    pub fn reason(&self) -> AbortReason {
        match self {
            Self::StepCouldNotBeBuilt { .. } => AbortReason::StepCouldNotBeBuilt,
            Self::PolicyCouldNotBeBuilt { .. } => AbortReason::PolicyCouldNotBeBuilt,
            Self::RequiredDataMissing { .. } => AbortReason::RequiredDataMissing,
            Self::WrongType { .. } => AbortReason::WrongType,
            Self::HookFailed { .. } => AbortReason::HookFailed,
            Self::ReporterFailed { .. } => AbortReason::ReporterFailed,
        }
    }

    /// The name of the step during which the journey was aborted, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::JourneyAbort;
    ///
    /// fn during_a_step(abort: &JourneyAbort) -> bool {
    ///     abort.step().is_some()
    /// }
    /// ```
    pub fn step(&self) -> Option<StepName> {
        match self {
            Self::StepCouldNotBeBuilt { step, .. } | Self::PolicyCouldNotBeBuilt { step, .. } => {
                Some(*step)
            }
            Self::RequiredDataMissing { missing } => missing.step(),
            Self::WrongType { requester, .. } => requester.step(),
            Self::HookFailed { step, .. } | Self::ReporterFailed { step, .. } => *step,
        }
    }

    /// The message of the error that caused the abort, when failing custom code did.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::JourneyAbort;
    ///
    /// fn caused_by_failing_code(abort: &JourneyAbort) -> bool {
    ///     abort.error().is_some()
    /// }
    /// ```
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::StepCouldNotBeBuilt { error, .. }
            | Self::PolicyCouldNotBeBuilt { error, .. }
            | Self::HookFailed { error, .. }
            | Self::ReporterFailed { error, .. } => Some(error),
            Self::RequiredDataMissing { .. } | Self::WrongType { .. } => None,
        }
    }
}

/// What a required request found missing: data under a key, or the failure's reason or the error
/// that a step hook requested from the step it acts on.
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
        /// The policy's name.
        policy: PolicyName,
        /// The hook that requested it.
        hook: StepHook,
        /// The name of the step it acts on.
        step: StepName,
    },
    /// The error, which did not end the attempt.
    #[non_exhaustive]
    Error {
        /// The policy's name.
        policy: PolicyName,
        /// The hook that requested it.
        hook: StepHook,
        /// The name of the step it acts on.
        step: StepName,
    },
}
impl MissingData {
    fn step(&self) -> Option<StepName> {
        match self {
            Self::Key { requester, .. } => requester.step(),
            Self::Reason { step, .. } | Self::Error { step, .. } => Some(*step),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::WorkflowHook;
    use crate::workflow::AdapterName;

    #[test]
    fn an_abort_while_data_was_resolved_names_the_step_it_was_for_but_no_error() {
        let abort = JourneyAbort::RequiredDataMissing {
            missing: MissingData::Key {
                key: "price".to_string(),
                requester: Requester::Adapter {
                    adapter: AdapterName::from("pricing"),
                    step: StepName::new("charge"),
                },
            },
        };
        assert_eq!(abort.reason(), AbortReason::RequiredDataMissing);
        assert_eq!(abort.step(), Some(StepName::new("charge")));
        assert_eq!(abort.error(), None);
    }

    #[test]
    fn a_required_request_for_a_missing_reason_names_the_step_the_hook_acts_on() {
        let abort = JourneyAbort::RequiredDataMissing {
            missing: MissingData::Reason {
                policy: PolicyName::from("alarm"),
                hook: StepHook::OnStepFailure,
                step: StepName::new("charge"),
            },
        };
        assert_eq!(abort.step(), Some(StepName::new("charge")));
        assert_eq!(abort.error(), None);
    }

    #[test]
    fn an_abort_by_a_workflow_hooks_request_names_no_step() {
        let abort = JourneyAbort::WrongType {
            key: "amount".to_string(),
            requester: Requester::WorkflowHook {
                policy: PolicyName::from("close"),
                hook: WorkflowHook::OnWorkflowSuccess,
            },
        };
        assert_eq!(abort.reason(), AbortReason::WrongType);
        assert_eq!(abort.step(), None);
    }

    #[test]
    fn an_abort_caused_by_failing_code_carries_the_errors_message() {
        let abort = JourneyAbort::PolicyCouldNotBeBuilt {
            step: StepName::new("ship"),
            policy: PolicyName::from("broken"),
            error: "no configuration".to_string(),
        };
        assert_eq!(abort.reason(), AbortReason::PolicyCouldNotBeBuilt);
        assert_eq!(abort.step(), Some(StepName::new("ship")));
        assert_eq!(abort.error(), Some("no configuration"));
    }
}
