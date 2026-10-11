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
    /// A step reported a failure that is not retriable, with this reason.
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
}
