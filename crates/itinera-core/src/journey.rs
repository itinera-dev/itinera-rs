//! Journeys, each one execution of a workflow: their identity, and how they can end.

use crate::step::Reason;

/// The identifier of one journey, unique to it.
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
pub struct JourneyId(String);

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
    use super::*;

    #[test]
    fn causes_and_abort_reasons_display_as_the_specification_writes_them() {
        assert_eq!(FailureCause::Failure.to_string(), "failure");
        assert_eq!(
            FailureCause::RetriesExhausted.to_string(),
            "retries exhausted"
        );
        assert_eq!(
            FailureCause::AbnormalTermination.to_string(),
            "abnormal termination"
        );
        assert_eq!(FailureCause::FailWorkflow.to_string(), "FailWorkflow");
        assert_eq!(
            AbortReason::StepCouldNotBeBuilt.to_string(),
            "step could not be built"
        );
        assert_eq!(
            AbortReason::PolicyCouldNotBeBuilt.to_string(),
            "policy could not be built"
        );
        assert_eq!(
            AbortReason::RequiredDataMissing.to_string(),
            "required data missing"
        );
        assert_eq!(AbortReason::WrongType.to_string(), "wrong type");
        assert_eq!(AbortReason::HookFailed.to_string(), "hook failed");
        assert_eq!(AbortReason::ReporterFailed.to_string(), "reporter failed");
    }
}
