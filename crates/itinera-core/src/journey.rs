//! Journeys, each one execution of a workflow: their identity, their data, and how they end.

use std::sync::Arc;

use crate::step::Reason;

mod abort;
mod access;
mod contributor;
mod data_bag;
mod failure;
#[cfg(test)]
mod fixtures;
mod result;

pub use abort::{Abort, MissingData, Requester};
pub use access::{DataBagAccess, Read};
pub use contributor::Contributor;
pub(crate) use contributor::{Contribution, Contributions};
pub use data_bag::DataBag;
pub use failure::Failure;
pub use result::{JourneyResult, JourneyStatus, StatusKind};

/// The identifier of one journey, unique to it. It is made when the journey's instance is
/// created, and cheap to clone.
///
/// # Examples
///
/// ```
/// use itinera::journey::JourneyId;
///
/// let id = JourneyId::from("order-42");
/// let text: &str = id.as_ref();
/// assert_eq!(text, "order-42");
/// assert_eq!(id.to_string(), "order-42");
/// assert_eq!(JourneyId::from(String::from("order-42")), id);
/// ```
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::Display,
    derive_more::From,
    derive_more::AsRef,
)]
#[from(forward)]
#[as_ref(forward)]
pub struct JourneyId(Arc<str>);

/// Why a journey failed.
///
/// It displays as the cause is named, for example `retries exhausted`.
///
/// # Examples
///
/// ```
/// use itinera::journey::FailureCause;
///
/// assert_eq!(FailureCause::AbnormalTermination.to_string(), "abnormal termination");
/// assert_eq!(FailureCause::FailWorkflow.to_string(), "FailWorkflow");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum FailureCause {
    /// A step reported a failure that is not retriable.
    #[display("failure")]
    Failure,
    /// A step's retry budget was spent.
    #[display("retries exhausted")]
    RetriesExhausted,
    /// A step ended in an abnormal termination that is not retried.
    #[display("abnormal termination")]
    AbnormalTermination,
    /// A hook returned `FailWorkflow`.
    FailWorkflow,
}

/// What ended a step's last attempt when its retry budget was spent: the reason of a failure,
/// or the error that ended it abnormally.
///
/// Events hold the error as its message, the default `E`; the result holds the error itself.
///
/// # Examples
///
/// ```
/// use itinera::journey::LastFailure;
///
/// fn describe(last: &LastFailure) -> &str {
///     match last {
///         LastFailure::Reason(reason) => reason.code(),
///         LastFailure::Error(message) => message,
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub enum LastFailure<E = String> {
    /// The attempt reported a retriable failure, with this reason.
    Reason(Reason),
    /// The attempt ended in an abnormal termination, with this error.
    Error(E),
}

/// Why a journey was aborted.
///
/// It displays as the reason is named, for example `reporter failed`. The reasons
/// `invalid lifecycle` and `not a value` cannot happen in Rust, so they are absent.
///
/// # Examples
///
/// ```
/// use itinera::journey::AbortReason;
///
/// assert_eq!(AbortReason::ReporterFailed.to_string(), "reporter failed");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum AbortReason {
    /// A step's constructor, or the input adapter resolving one of its inputs, failed.
    #[display("step could not be built")]
    StepCouldNotBeBuilt,
    /// A step policy failed while being built for an attempt.
    #[display("policy could not be built")]
    PolicyCouldNotBeBuilt,
    /// A required request has no value.
    #[display("required data missing")]
    RequiredDataMissing,
    /// A requested value has the wrong type.
    #[display("wrong type")]
    WrongType,
    /// A hook, or a role operation it called, failed.
    #[display("hook failed")]
    HookFailed,
    /// A reporter, or a dispatcher while dispatching, failed.
    #[display("reporter failed")]
    ReporterFailed,
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::cause_failure(&FailureCause::Failure, "failure")]
    #[case::cause_retries_exhausted(&FailureCause::RetriesExhausted, "retries exhausted")]
    #[case::cause_abnormal_termination(&FailureCause::AbnormalTermination, "abnormal termination")]
    #[case::cause_fail_workflow(&FailureCause::FailWorkflow, "FailWorkflow")]
    #[case::reason_step_could_not_be_built(&AbortReason::StepCouldNotBeBuilt, "step could not be built")]
    #[case::reason_policy_could_not_be_built(
        &AbortReason::PolicyCouldNotBeBuilt,
        "policy could not be built"
    )]
    #[case::reason_required_data_missing(&AbortReason::RequiredDataMissing, "required data missing")]
    #[case::reason_wrong_type(&AbortReason::WrongType, "wrong type")]
    #[case::reason_hook_failed(&AbortReason::HookFailed, "hook failed")]
    #[case::reason_reporter_failed(&AbortReason::ReporterFailed, "reporter failed")]
    fn causes_and_abort_reasons_display_as_the_specification_writes_them(
        #[case] name: &dyn fmt::Display,
        #[case] written: &str,
    ) {
        assert_eq!(name.to_string(), written);
    }
}
