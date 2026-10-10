//! What the step's own rule decides after an attempt that did not succeed: another attempt, or
//! giving the step up. It reads only the attempt's end, the attempt number and the step
//! descriptor, and changes nothing.

use std::num::NonZeroU32;

use crate::error::Error;
use crate::event::{GiveUpCause, JourneyFailure, RetryCause};
use crate::journey::{Failure, LastFailure};
use crate::step::{Reason, StepDescriptor};

/// How an attempt that did not succeed ended, with what it carries.
pub(crate) enum Failed {
    Failure(Reason),
    RetriableFailure(Reason),
    AbnormalTermination(Error),
}

/// What the step's own rule decides, before a hook may decide otherwise.
#[derive(Debug)]
pub(crate) enum Decision {
    /// The step is attempted again.
    Retry(RetryCause),
    /// The step will not be attempted again.
    GiveUp(GivenUp),
}

/// Why the step's own rule gives it up, with what its last attempt carries.
#[derive(Debug)]
pub(crate) enum GivenUp {
    Failure(Reason),
    AbnormalTermination(Error),
    RetriesExhausted(LastFailure<Error>),
}

/// Decides what follows the attempt numbered `attempt` of the step, which ended as `failed`.
pub(crate) fn decide<M>(failed: Failed, attempt: NonZeroU32, step: &StepDescriptor<M>) -> Decision {
    match failed {
        Failed::Failure(reason) => Decision::GiveUp(GivenUp::Failure(reason)),
        Failed::RetriableFailure(reason) => within_budget(
            RetryCause::RetriableFailure,
            LastFailure::Reason(reason),
            attempt,
            step,
        ),
        Failed::AbnormalTermination(error) if step.retries_abnormal_termination() => within_budget(
            RetryCause::AbnormalTermination,
            LastFailure::Error(error),
            attempt,
            step,
        ),
        Failed::AbnormalTermination(error) => Decision::GiveUp(GivenUp::AbnormalTermination(error)),
    }
}

/// Retries for the cause while the budget allows another attempt, and otherwise gives the step
/// up with its last failure.
fn within_budget<M>(
    cause: RetryCause,
    last: LastFailure<Error>,
    attempt: NonZeroU32,
    step: &StepDescriptor<M>,
) -> Decision {
    if step.budget_allows_after(attempt) {
        Decision::Retry(cause)
    } else {
        Decision::GiveUp(GivenUp::RetriesExhausted(last))
    }
}

impl GivenUp {
    /// The cause `step_given_up` reports.
    pub(crate) fn cause(&self) -> GiveUpCause {
        match self {
            Self::Failure(_) => GiveUpCause::Failure,
            Self::AbnormalTermination(_) => GiveUpCause::AbnormalTermination,
            Self::RetriesExhausted(_) => GiveUpCause::RetriesExhausted,
        }
    }

    /// How the journey fails when no hook decides otherwise: as `journey_failed` reports it,
    /// with the error's message, and as the result holds it, with the error itself.
    pub(crate) fn into_failure(self) -> (JourneyFailure, Failure) {
        match self {
            Self::Failure(reason) => (
                JourneyFailure::Failure(reason.clone()),
                Failure::Failure(reason),
            ),
            Self::AbnormalTermination(error) => (
                JourneyFailure::AbnormalTermination(error.to_string()),
                Failure::AbnormalTermination(error),
            ),
            Self::RetriesExhausted(last) => (
                JourneyFailure::RetriesExhausted(reported(&last)),
                Failure::RetriesExhausted(last),
            ),
        }
    }
}

/// The last failure as events carry it, with the error's message.
fn reported(last: &LastFailure<Error>) -> LastFailure {
    match last {
        LastFailure::Reason(reason) => LastFailure::Reason(reason.clone()),
        LastFailure::Error(error) => LastFailure::Error(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::journey::FailureCause;
    use crate::step::{Outcome, StepName};

    fn charge() -> StepDescriptor {
        StepDescriptor::new(StepName::new("charge"), || Ok(Outcome::success()))
    }

    fn declined() -> Failed {
        Failed::Failure(Reason::new("declined"))
    }

    fn timed_out() -> Failed {
        Failed::RetriableFailure(Reason::new("timeout"))
    }

    fn crashed() -> Failed {
        Failed::AbnormalTermination(Error::msg("the gateway crashed"))
    }

    fn attempt(number: u32) -> NonZeroU32 {
        NonZeroU32::new(number).unwrap()
    }

    /// A decision, as the tests compare it.
    #[derive(Debug, PartialEq)]
    enum Decided {
        Retry(RetryCause),
        GiveUp(FailureCause),
    }

    fn decided(decision: Decision) -> Decided {
        match decision {
            Decision::Retry(cause) => Decided::Retry(cause),
            Decision::GiveUp(given_up) => Decided::GiveUp(given_up.into_failure().1.cause()),
        }
    }

    #[rstest]
    #[case::a_failure_with_budget_left(
        declined(),
        1,
        charge().retry_budget(3),
        Decided::GiveUp(FailureCause::Failure)
    )]
    #[case::a_retriable_failure_with_budget_left(
        timed_out(),
        1,
        charge().retry_budget(1),
        Decided::Retry(RetryCause::RetriableFailure)
    )]
    #[case::a_retriable_failure_on_the_last_attempt_the_budget_allows(
        timed_out(),
        2,
        charge().retry_budget(1),
        Decided::GiveUp(FailureCause::RetriesExhausted)
    )]
    #[case::a_retriable_failure_without_a_budget(
        timed_out(),
        1,
        charge(),
        Decided::GiveUp(FailureCause::RetriesExhausted)
    )]
    #[case::an_abnormal_termination_the_step_does_not_retry(
        crashed(),
        1,
        charge().retry_budget(3),
        Decided::GiveUp(FailureCause::AbnormalTermination)
    )]
    #[case::an_abnormal_termination_the_step_retries_with_budget_left(
        crashed(),
        3,
        charge().retry_budget(3).abnormal_termination_retriable(),
        Decided::Retry(RetryCause::AbnormalTermination)
    )]
    #[case::an_abnormal_termination_the_step_retries_on_the_last_attempt_the_budget_allows(
        crashed(),
        4,
        charge().retry_budget(3).abnormal_termination_retriable(),
        Decided::GiveUp(FailureCause::RetriesExhausted)
    )]
    #[case::a_retriable_failure_before_the_last_attempt_the_largest_budget_allows(
        timed_out(),
        u32::from(u16::MAX),
        charge().retry_budget(u16::MAX),
        Decided::Retry(RetryCause::RetriableFailure)
    )]
    #[case::a_retriable_failure_on_the_last_attempt_the_largest_budget_allows(
        timed_out(),
        u32::from(u16::MAX) + 1,
        charge().retry_budget(u16::MAX),
        Decided::GiveUp(FailureCause::RetriesExhausted)
    )]
    fn a_step_is_retried_only_for_a_retriable_end_while_its_budget_allows_another_attempt(
        #[case] failed: Failed,
        #[case] number: u32,
        #[case] step: StepDescriptor,
        #[case] expected: Decided,
    ) {
        assert_eq!(decided(decide(failed, attempt(number), &step)), expected);
    }

    #[test]
    fn retries_exhausted_carries_what_ended_the_last_attempt() {
        let Decision::GiveUp(given_up) = decide(
            crashed(),
            attempt(1),
            &charge().abnormal_termination_retriable(),
        ) else {
            panic!("the step was retried without a budget");
        };
        let (reported, failure) = given_up.into_failure();
        assert!(matches!(
            reported,
            JourneyFailure::RetriesExhausted(LastFailure::Error(message)) if message == "the gateway crashed"
        ));
        assert!(matches!(
            failure,
            Failure::RetriesExhausted(LastFailure::Error(error)) if error.to_string() == "the gateway crashed"
        ));
    }
}
