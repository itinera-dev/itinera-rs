//! Why a journey failed, as events tell it.

use super::DecidingHook;
use crate::journey::{FailureCause, LastFailure};
use crate::step::Reason;

/// Why a journey failed, with what the cause carries and who decided it.
///
/// # Examples
///
/// ```
/// use itinera::event::JourneyFailure;
/// use itinera::journey::FailureCause;
///
/// fn by_a_hook(failure: &JourneyFailure) -> bool {
///     failure.cause() == FailureCause::FailWorkflow
/// }
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum JourneyFailure {
    /// The step reported a failure that is not retriable, with this reason.
    Failure(Reason),
    /// The step's retry budget was spent.
    RetriesExhausted(LastFailure),
    /// The step ended in an abnormal termination that is not retried, with this error message.
    AbnormalTermination(String),
    /// A hook of the step returned `FailWorkflow`.
    #[non_exhaustive]
    FailWorkflow {
        /// The hook that returned it.
        decided_by: DecidingHook,
        /// The reason it carried.
        reason: Reason,
    },
}
impl JourneyFailure {
    /// The failure's cause.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::JourneyFailure;
    /// use itinera::journey::FailureCause;
    ///
    /// fn retried(failure: &JourneyFailure) -> bool {
    ///     failure.cause() == FailureCause::RetriesExhausted
    /// }
    /// ```
    pub fn cause(&self) -> FailureCause {
        match self {
            Self::Failure(_) => FailureCause::Failure,
            Self::RetriesExhausted(_) => FailureCause::RetriesExhausted,
            Self::AbnormalTermination(_) => FailureCause::AbnormalTermination,
            Self::FailWorkflow { .. } => FailureCause::FailWorkflow,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::policy::{PolicyName, StepHook};

    fn close(hook: StepHook) -> DecidingHook {
        DecidingHook {
            policy: PolicyName::from("close"),
            hook,
        }
    }

    #[rstest]
    #[case::failure(
        JourneyFailure::Failure(Reason::new("declined")),
        FailureCause::Failure
    )]
    #[case::retries_exhausted(
        JourneyFailure::RetriesExhausted(LastFailure::Error("timeout".to_string())),
        FailureCause::RetriesExhausted
    )]
    #[case::abnormal_termination(
        JourneyFailure::AbnormalTermination("boom".to_string()),
        FailureCause::AbnormalTermination
    )]
    #[case::fail_workflow(
        JourneyFailure::FailWorkflow {
            decided_by: close(StepHook::OnStepSuccess),
            reason: Reason::new("fraud"),
        },
        FailureCause::FailWorkflow
    )]
    fn a_journey_failure_names_its_cause(
        #[case] failure: JourneyFailure,
        #[case] cause: FailureCause,
    ) {
        assert_eq!(failure.cause(), cause);
    }
}
