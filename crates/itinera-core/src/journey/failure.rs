//! Why a journey failed, as its result tells it.

use super::{FailureCause, LastFailure};
use crate::error::Error;
use crate::step::Reason;

/// Why a journey failed: one variant per cause, each holding the reason when a step or hook gave
/// one, and the error when an error ended the last attempt.
///
/// # Examples
///
/// ```
/// use itinera::journey::Failure;
///
/// fn code(failure: &Failure) -> Option<&str> {
///     match failure {
///         Failure::Failure(reason) | Failure::FailWorkflow(reason) => Some(reason.code()),
///         _ => None,
///     }
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum Failure {
    /// A step without a retry budget reported a failure, with this reason.
    Failure(Reason),
    /// A step's retry budget was spent, and this ended its last attempt.
    RetriesExhausted(LastFailure<Error>),
    /// A step ended in an abnormal termination that is not retried, with this error.
    AbnormalTermination(Error),
    /// A hook returned `FailWorkflow`, with this reason.
    FailWorkflow(Reason),
}

impl Failure {
    /// The failure's cause.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{Failure, FailureCause};
    ///
    /// fn decided_by_a_hook(failure: &Failure) -> bool {
    ///     failure.cause() == FailureCause::FailWorkflow
    /// }
    /// ```
    pub fn cause(&self) -> FailureCause {
        match self {
            Self::Failure(_) => FailureCause::Failure,
            Self::RetriesExhausted(_) => FailureCause::RetriesExhausted,
            Self::AbnormalTermination(_) => FailureCause::AbnormalTermination,
            Self::FailWorkflow(_) => FailureCause::FailWorkflow,
        }
    }

    /// The reason the failure carries: a step's, the one `FailWorkflow` carried, or that of the
    /// last attempt when the retries ran out after a failure. `None` when an error ended the last
    /// attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::Failure;
    ///
    /// fn code(failure: &Failure) -> Option<&str> {
    ///     failure.reason().map(|reason| reason.code())
    /// }
    /// ```
    pub fn reason(&self) -> Option<&Reason> {
        match self {
            Self::Failure(reason)
            | Self::FailWorkflow(reason)
            | Self::RetriesExhausted(LastFailure::Reason(reason)) => Some(reason),
            Self::AbnormalTermination(_) | Self::RetriesExhausted(LastFailure::Error(_)) => None,
        }
    }

    /// The error that ended the last attempt, when an abnormal termination did, whether or not
    /// retries were tried first. `None` when the failure carries a reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::Failure;
    ///
    /// fn crashed(failure: &Failure) -> Option<String> {
    ///     failure.error().map(ToString::to_string)
    /// }
    /// ```
    pub fn error(&self) -> Option<&Error> {
        match self {
            Self::AbnormalTermination(error)
            | Self::RetriesExhausted(LastFailure::Error(error)) => Some(error),
            Self::Failure(_)
            | Self::FailWorkflow(_)
            | Self::RetriesExhausted(LastFailure::Reason(_)) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::failure(Failure::Failure(Reason::new("declined")), FailureCause::Failure)]
    #[case::retries_exhausted(
        Failure::RetriesExhausted(LastFailure::Error(Error::msg("timeout"))),
        FailureCause::RetriesExhausted
    )]
    #[case::abnormal_termination(
        Failure::AbnormalTermination(Error::msg("timeout")),
        FailureCause::AbnormalTermination
    )]
    #[case::fail_workflow(
        Failure::FailWorkflow(Reason::new("fraud")),
        FailureCause::FailWorkflow
    )]
    fn a_failure_names_its_cause(#[case] failure: Failure, #[case] cause: FailureCause) {
        assert_eq!(failure.cause(), cause);
    }

    #[rstest]
    #[case::failure(Failure::Failure(Reason::new("declined")), Some("declined"))]
    #[case::fail_workflow(Failure::FailWorkflow(Reason::new("fraud")), Some("fraud"))]
    #[case::retries_exhausted_after_a_failure(
        Failure::RetriesExhausted(LastFailure::Reason(Reason::new("declined"))),
        Some("declined")
    )]
    #[case::retries_exhausted_after_an_error(
        Failure::RetriesExhausted(LastFailure::Error(Error::msg("timeout"))),
        None
    )]
    #[case::abnormal_termination(Failure::AbnormalTermination(Error::msg("timeout")), None)]
    fn a_failure_carries_the_reason_of_whatever_ended_it_with_one(
        #[case] failure: Failure,
        #[case] code: Option<&str>,
    ) {
        assert_eq!(failure.reason().map(Reason::code), code);
    }

    #[rstest]
    #[case::failure(Failure::Failure(Reason::new("declined")), None)]
    #[case::fail_workflow(Failure::FailWorkflow(Reason::new("fraud")), None)]
    #[case::retries_exhausted_after_a_failure(
        Failure::RetriesExhausted(LastFailure::Reason(Reason::new("declined"))),
        None
    )]
    #[case::retries_exhausted_after_an_error(
        Failure::RetriesExhausted(LastFailure::Error(Error::msg("timeout"))),
        Some("timeout")
    )]
    #[case::abnormal_termination(
        Failure::AbnormalTermination(Error::msg("closed")),
        Some("closed")
    )]
    fn a_failure_carries_the_error_that_ended_the_last_attempt(
        #[case] failure: Failure,
        #[case] message: Option<&str>,
    ) {
        assert_eq!(failure.error().map(ToString::to_string).as_deref(), message);
    }
}
