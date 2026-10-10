//! What the step's own rule decides after an attempt that did not succeed: another attempt, or
//! giving the step up. It reads only the attempt's end, the attempt number and the step
//! descriptor, and changes nothing.

use std::num::NonZeroU32;

use crate::error::Error;
use crate::event::{GiveUpCause, JourneyFailure};
use crate::journey::{Failure, LastFailure};
use crate::policy::{RetryCause, StepFailureCause};
use crate::step::{Reason, StepDescriptor};

/// How an attempt that did not succeed ended, with what it carries.
#[derive(Debug)]
pub(crate) enum Failed {
    Failure(Reason),
    RetriableFailure(Reason),
    AbnormalTermination(Error),
}

/// What the step's own rule decides, before a hook may decide otherwise.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The step is attempted again.
    Retry(RetryCause),
    /// The step will not be attempted again.
    GiveUp(StepFailureCause),
}

/// Decides what follows the attempt numbered `attempt` of the step, which ended as `failed`.
pub(crate) fn decide<W, M>(
    failed: &Failed,
    attempt: NonZeroU32,
    step: &StepDescriptor<W, M>,
) -> Decision {
    match failed {
        Failed::Failure(_) => Decision::GiveUp(StepFailureCause::Failure),
        Failed::RetriableFailure(_) => within_budget(RetryCause::RetriableFailure, attempt, step),
        Failed::AbnormalTermination(_) if step.retries_abnormal_termination() => {
            within_budget(RetryCause::AbnormalTermination, attempt, step)
        }
        Failed::AbnormalTermination(_) => Decision::GiveUp(StepFailureCause::AbnormalTermination),
    }
}

/// Retries for the cause while the budget allows another attempt, and otherwise gives the step
/// up.
fn within_budget<W, M>(
    cause: RetryCause,
    attempt: NonZeroU32,
    step: &StepDescriptor<W, M>,
) -> Decision {
    if step.budget_allows_after(attempt) {
        Decision::Retry(cause)
    } else {
        Decision::GiveUp(StepFailureCause::RetriesExhausted)
    }
}

/// The cause `step_given_up` reports when the step's own rule gave it up.
pub(crate) fn given_up(cause: StepFailureCause) -> GiveUpCause {
    match cause {
        StepFailureCause::Failure => GiveUpCause::Failure,
        StepFailureCause::AbnormalTermination => GiveUpCause::AbnormalTermination,
        StepFailureCause::RetriesExhausted => GiveUpCause::RetriesExhausted,
    }
}

impl Failed {
    /// The reason the attempt reported, if it reported a failure.
    pub(crate) fn reason(&self) -> Option<&Reason> {
        match self {
            Self::Failure(reason) | Self::RetriableFailure(reason) => Some(reason),
            Self::AbnormalTermination(_) => None,
        }
    }

    /// The error that ended the attempt, if it ended in an abnormal termination.
    pub(crate) fn error(&self) -> Option<&Error> {
        match self {
            Self::AbnormalTermination(error) => Some(error),
            Self::Failure(_) | Self::RetriableFailure(_) => None,
        }
    }

    /// How the journey fails when the step was given up for `cause` and no hook decides
    /// otherwise: as `journey_failed` reports it, with the error's message, and as the result
    /// holds it, with the error itself.
    pub(crate) fn into_failure(self, cause: StepFailureCause) -> (JourneyFailure, Failure) {
        match (cause, self) {
            (StepFailureCause::RetriesExhausted, failed) => {
                let last = failed.into_last();
                (
                    JourneyFailure::RetriesExhausted(reported(&last)),
                    Failure::RetriesExhausted(last),
                )
            }
            (_, Self::AbnormalTermination(error)) => (
                JourneyFailure::AbnormalTermination(error.to_string()),
                Failure::AbnormalTermination(error),
            ),
            (_, Self::Failure(reason) | Self::RetriableFailure(reason)) => (
                JourneyFailure::Failure(reason.clone()),
                Failure::Failure(reason),
            ),
        }
    }

    /// What ended the attempt, as the last of a step whose retry budget is spent.
    fn into_last(self) -> LastFailure<Error> {
        match self {
            Self::Failure(reason) | Self::RetriableFailure(reason) => LastFailure::Reason(reason),
            Self::AbnormalTermination(error) => LastFailure::Error(error),
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
    use crate::step::{Outcome, StepName};

    fn charge() -> StepDescriptor<()> {
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

    #[rstest]
    #[case::a_failure_with_budget_left(
        declined(),
        1,
        charge().retry_budget(3),
        Decision::GiveUp(StepFailureCause::Failure)
    )]
    #[case::a_retriable_failure_with_budget_left(
        timed_out(),
        1,
        charge().retry_budget(1),
        Decision::Retry(RetryCause::RetriableFailure)
    )]
    #[case::a_retriable_failure_on_the_last_attempt_the_budget_allows(
        timed_out(),
        2,
        charge().retry_budget(1),
        Decision::GiveUp(StepFailureCause::RetriesExhausted)
    )]
    #[case::a_retriable_failure_without_a_budget(
        timed_out(),
        1,
        charge(),
        Decision::GiveUp(StepFailureCause::RetriesExhausted)
    )]
    #[case::an_abnormal_termination_the_step_does_not_retry(
        crashed(),
        1,
        charge().retry_budget(3),
        Decision::GiveUp(StepFailureCause::AbnormalTermination)
    )]
    #[case::an_abnormal_termination_the_step_retries_with_budget_left(
        crashed(),
        3,
        charge().retry_budget(3).abnormal_termination_retriable(),
        Decision::Retry(RetryCause::AbnormalTermination)
    )]
    #[case::an_abnormal_termination_the_step_retries_on_the_last_attempt_the_budget_allows(
        crashed(),
        4,
        charge().retry_budget(3).abnormal_termination_retriable(),
        Decision::GiveUp(StepFailureCause::RetriesExhausted)
    )]
    #[case::a_retriable_failure_before_the_last_attempt_the_largest_budget_allows(
        timed_out(),
        u32::from(u16::MAX),
        charge().retry_budget(u16::MAX),
        Decision::Retry(RetryCause::RetriableFailure)
    )]
    #[case::a_retriable_failure_on_the_last_attempt_the_largest_budget_allows(
        timed_out(),
        u32::from(u16::MAX) + 1,
        charge().retry_budget(u16::MAX),
        Decision::GiveUp(StepFailureCause::RetriesExhausted)
    )]
    fn a_step_is_retried_only_for_a_retriable_end_while_its_budget_allows_another_attempt(
        #[case] failed: Failed,
        #[case] number: u32,
        #[case] step: StepDescriptor<()>,
        #[case] expected: Decision,
    ) {
        assert_eq!(decide(&failed, attempt(number), &step), expected);
    }

    #[test]
    fn retries_exhausted_carries_what_ended_the_last_attempt() {
        let (reported, failure) = crashed().into_failure(StepFailureCause::RetriesExhausted);
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
