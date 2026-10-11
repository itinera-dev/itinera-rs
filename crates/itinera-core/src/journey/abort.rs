//! Why a journey was aborted, as its result tells it, with what a request found missing and who
//! made it.

use super::AbortReason;
use crate::error::Error;
use crate::policy::{PolicyName, StepHook, WorkflowHook};
use crate::workflow::AdapterName;

/// Why a journey was aborted: one variant per abort reason, each holding its details, and the
/// error when failing custom code caused it.
///
/// # Examples
///
/// ```
/// use itinera::journey::Abort;
///
/// fn caused_by(abort: &Abort) -> Option<String> {
///     match abort {
///         Abort::HookFailed(error) | Abort::ReporterFailed(error) => Some(error.to_string()),
///         _ => None,
///     }
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum Abort {
    /// A step's constructor, or the input adapter resolving one of its inputs, failed with this
    /// error.
    StepCouldNotBeBuilt(Error),
    /// A step policy failed while being built for an attempt.
    #[non_exhaustive]
    PolicyCouldNotBeBuilt {
        /// The policy's name.
        policy: PolicyName,
        /// The error it failed with.
        error: Error,
    },
    /// A required request has no value.
    RequiredDataMissing(MissingData),
    /// A requested value has the wrong type.
    #[non_exhaustive]
    WrongType {
        /// The key that was requested.
        key: String,
        /// Who requested it.
        requester: Requester,
    },
    /// A hook, or a role operation it called, failed with this error.
    HookFailed(Error),
    /// A reporter, or a dispatcher while dispatching, failed with this error.
    ReporterFailed(Error),
}

impl Abort {
    /// The abort's reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{Abort, AbortReason};
    ///
    /// fn caused_by_a_reporter(abort: &Abort) -> bool {
    ///     abort.reason() == AbortReason::ReporterFailed
    /// }
    /// ```
    pub fn reason(&self) -> AbortReason {
        match self {
            Self::StepCouldNotBeBuilt(_) => AbortReason::StepCouldNotBeBuilt,
            Self::PolicyCouldNotBeBuilt { .. } => AbortReason::PolicyCouldNotBeBuilt,
            Self::RequiredDataMissing(_) => AbortReason::RequiredDataMissing,
            Self::WrongType { .. } => AbortReason::WrongType,
            Self::HookFailed(_) => AbortReason::HookFailed,
            Self::ReporterFailed(_) => AbortReason::ReporterFailed,
        }
    }
}

/// What a required request found missing: data under a key, or the failure's reason or the error
/// that a step hook requested from the step it acts on.
///
/// It is the result's counterpart of [`event::MissingData`](crate::event::MissingData), without
/// the step's name, since the result names no step.
///
/// # Examples
///
/// ```
/// use itinera::journey::MissingData;
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
        /// Who requested it.
        requester: Requester,
    },
    /// The failure's reason, which the attempt did not report.
    #[non_exhaustive]
    Reason {
        /// The policy's name.
        policy: PolicyName,
        /// The hook that requested it.
        hook: StepHook,
    },
    /// The error, which did not end the attempt.
    #[non_exhaustive]
    Error {
        /// The policy's name.
        policy: PolicyName,
        /// The hook that requested it.
        hook: StepHook,
    },
}

/// Who made a request whose data could not be resolved: a step for one of its inputs, an input
/// adapter resolving a step's input, a step hook, or a workflow hook.
///
/// It is the result's counterpart of [`event::Requester`](crate::event::Requester), without the
/// step's name, since the result names no step.
///
/// # Examples
///
/// ```
/// use itinera::journey::Requester;
/// use itinera::policy::PolicyName;
///
/// fn policy(requester: &Requester) -> Option<PolicyName> {
///     match requester {
///         Requester::StepHook { policy, .. } | Requester::WorkflowHook { policy, .. } => {
///             Some(*policy)
///         }
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Requester {
    /// A step, for one of its inputs.
    Step,
    /// An input adapter, for an input of a step.
    #[non_exhaustive]
    Adapter {
        /// The adapter's name.
        adapter: AdapterName,
    },
    /// A step hook.
    #[non_exhaustive]
    StepHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: StepHook,
    },
    /// A workflow hook.
    #[non_exhaustive]
    WorkflowHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: WorkflowHook,
    },
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn missing_amount() -> MissingData {
        MissingData::Key {
            key: "amount".to_string(),
            requester: Requester::Step,
        }
    }

    #[rstest]
    #[case::step_could_not_be_built(
        Abort::StepCouldNotBeBuilt(Error::msg("closed")),
        AbortReason::StepCouldNotBeBuilt
    )]
    #[case::policy_could_not_be_built(
        Abort::PolicyCouldNotBeBuilt {
            policy: PolicyName::from("audit"),
            error: Error::msg("closed"),
        },
        AbortReason::PolicyCouldNotBeBuilt
    )]
    #[case::required_data_missing(
        Abort::RequiredDataMissing(missing_amount()),
        AbortReason::RequiredDataMissing
    )]
    #[case::wrong_type(
        Abort::WrongType {
            key: "amount".to_string(),
            requester: Requester::Step,
        },
        AbortReason::WrongType
    )]
    #[case::hook_failed(Abort::HookFailed(Error::msg("closed")), AbortReason::HookFailed)]
    #[case::reporter_failed(
        Abort::ReporterFailed(Error::msg("closed")),
        AbortReason::ReporterFailed
    )]
    fn an_abort_names_its_reason(#[case] abort: Abort, #[case] reason: AbortReason) {
        assert_eq!(abort.reason(), reason);
    }
}
