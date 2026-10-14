//! What the step's own rule decides after an attempt that did not succeed: rescuing the step,
//! another attempt, or giving the step up. It reads only the attempt's end, the attempt number
//! and the step descriptor, and changes nothing.

use std::num::NonZeroU32;

use crate::error::Error;
use crate::event::{GiveUpCause, JourneyFailure, RescueCause};
use crate::journey::{Failure, LastFailure};
use crate::policy::{RetryCause, StepFailureCause};
use crate::step::{Reason, StepDescriptor};

/// How an attempt that did not succeed ended, with what it carries.
#[derive(Debug)]
pub(crate) enum Failed {
    Failure(Reason),
    AbnormalTermination(Error),
}

/// What the step's own rule decides about a step described by a `StepDescriptor<W, M>` that lives
/// for `'s`, before a hook may decide otherwise.
#[derive(derive_more::Debug)]
pub(crate) enum Decision<'s, W, M> {
    /// The step's rescue step runs, then the step is attempted again.
    #[debug("Rescue({}, {_1:?})", _0.name())]
    Rescue(&'s StepDescriptor<W, M>, RescueCause),
    /// The step is attempted again.
    Retry(RetryCause),
    /// The step will not be attempted again.
    GiveUp(StepFailureCause),
}

/// Decides what follows the attempt numbered `attempt` of the step, which ended as `failed`.
///
/// A step with a rescue step is rescued after its first attempt. It has no retry budget, so the
/// attempt after the rescue is its last. Any other step is retried while its budget allows,
/// unless the attempt ended in an abnormal termination and the step does not retry those.
pub(crate) fn decide<'s, W, M>(
    failed: &Failed,
    attempt: NonZeroU32,
    step: &'s StepDescriptor<W, M>,
) -> Decision<'s, W, M> {
    match (step.rescue_step(), failed) {
        (Some(rescue), _) if attempt == NonZeroU32::MIN => {
            Decision::Rescue(rescue, rescue_cause(failed))
        }
        (_, Failed::Failure(_)) => within_budget(RetryCause::Failure, attempt, step),
        (_, Failed::AbnormalTermination(_)) if step.retries_abnormal_termination() => {
            within_budget(RetryCause::AbnormalTermination, attempt, step)
        }
        (_, Failed::AbnormalTermination(_)) => {
            Decision::GiveUp(StepFailureCause::AbnormalTermination)
        }
    }
}

/// Retries for the cause while the budget allows another attempt. Otherwise gives the step up:
/// with `retries exhausted` once a budget is spent, and for the cause itself when the step has no
/// budget, since nothing was retried.
fn within_budget<'s, W, M>(
    cause: RetryCause,
    attempt: NonZeroU32,
    step: &StepDescriptor<W, M>,
) -> Decision<'s, W, M> {
    match (
        step.budget_allows_after(attempt),
        step.allows_retries(),
        cause,
    ) {
        (true, _, _) => Decision::Retry(cause),
        (false, true, _) => Decision::GiveUp(StepFailureCause::RetriesExhausted),
        (false, false, RetryCause::Failure) => Decision::GiveUp(StepFailureCause::Failure),
        (false, false, RetryCause::AbnormalTermination) => {
            Decision::GiveUp(StepFailureCause::AbnormalTermination)
        }
    }
}

/// How the attempt of a step that is rescued failed, as `step_rescuing` reports it.
fn rescue_cause(failed: &Failed) -> RescueCause {
    match failed {
        Failed::Failure(_) => RescueCause::Failure,
        Failed::AbnormalTermination(_) => RescueCause::AbnormalTermination,
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
            Self::Failure(reason) => Some(reason),
            Self::AbnormalTermination(_) => None,
        }
    }

    /// The error that ended the attempt, if it ended in an abnormal termination.
    pub(crate) fn error(&self) -> Option<&Error> {
        match self {
            Self::AbnormalTermination(error) => Some(error),
            Self::Failure(_) => None,
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
            (_, Self::Failure(reason)) => (
                JourneyFailure::Failure(reason.clone()),
                Failure::Failure(reason),
            ),
        }
    }

    /// What ended the attempt, as the last of a step whose retry budget is spent.
    fn into_last(self) -> LastFailure<Error> {
        match self {
            Self::Failure(reason) => LastFailure::Reason(reason),
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
impl<W, M> PartialEq for Decision<'_, W, M> {
    /// Two decisions to rescue are the same when they name the same rescue step, for the same
    /// cause.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Rescue(rescue, cause), Self::Rescue(other, other_cause)) => {
                rescue.name() == other.name() && cause == other_cause
            }
            (Self::Retry(cause), Self::Retry(other)) => cause == other,
            (Self::GiveUp(cause), Self::GiveUp(other)) => cause == other,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::mode::Synchronous;
    use crate::step::{Outcome, RescueOutcome, StepName};

    fn charge() -> StepDescriptor<()> {
        StepDescriptor::new(StepName::new("charge"), || Ok(Outcome::success()))
    }

    fn rescued_charge() -> StepDescriptor<()> {
        charge().rescue(|| Ok(RescueOutcome::success()))
    }

    fn declined() -> Failed {
        Failed::Failure(Reason::new("declined"))
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
        charge().retry_budget(1),
        Decision::Retry(RetryCause::Failure)
    )]
    #[case::a_failure_on_the_last_attempt_the_budget_allows(
        declined(),
        2,
        charge().retry_budget(1),
        Decision::GiveUp(StepFailureCause::RetriesExhausted)
    )]
    #[case::a_failure_without_a_budget(
        declined(),
        1,
        charge(),
        Decision::GiveUp(StepFailureCause::Failure)
    )]
    #[case::an_abnormal_termination_with_budget_left(
        crashed(),
        3,
        charge().retry_budget(3),
        Decision::Retry(RetryCause::AbnormalTermination)
    )]
    #[case::an_abnormal_termination_on_the_last_attempt_the_budget_allows(
        crashed(),
        4,
        charge().retry_budget(3),
        Decision::GiveUp(StepFailureCause::RetriesExhausted)
    )]
    #[case::an_abnormal_termination_without_a_budget(
        crashed(),
        1,
        charge(),
        Decision::GiveUp(StepFailureCause::AbnormalTermination)
    )]
    #[case::an_abnormal_termination_of_a_step_that_does_not_retry_them(
        crashed(),
        1,
        charge().retry_budget(3).no_retry_on_abnormal_termination(),
        Decision::GiveUp(StepFailureCause::AbnormalTermination)
    )]
    #[case::a_failure_before_the_last_attempt_the_largest_budget_allows(
        declined(),
        u32::from(u16::MAX),
        charge().retry_budget(u16::MAX),
        Decision::Retry(RetryCause::Failure)
    )]
    #[case::a_failure_on_the_last_attempt_the_largest_budget_allows(
        declined(),
        u32::from(u16::MAX) + 1,
        charge().retry_budget(u16::MAX),
        Decision::GiveUp(StepFailureCause::RetriesExhausted)
    )]
    fn a_step_is_retried_while_its_budget_allows_another_attempt(
        #[case] failed: Failed,
        #[case] number: u32,
        #[case] step: StepDescriptor<()>,
        #[case] expected: Decision<'_, (), Synchronous>,
    ) {
        assert_eq!(decide(&failed, attempt(number), &step), expected);
    }

    #[rstest]
    #[case::a_failure(declined(), rescued_charge(), RescueCause::Failure)]
    #[case::an_abnormal_termination(crashed(), rescued_charge(), RescueCause::AbnormalTermination)]
    #[case::an_abnormal_termination_the_step_does_not_retry(
        crashed(),
        rescued_charge().no_retry_on_abnormal_termination(),
        RescueCause::AbnormalTermination
    )]
    fn a_step_with_a_rescue_step_is_rescued_after_its_first_attempt_however_it_failed(
        #[case] failed: Failed,
        #[case] charge: StepDescriptor<()>,
        #[case] cause: RescueCause,
    ) {
        let rescue = charge.rescue_step().unwrap();

        assert_eq!(rescue.name(), StepName::new("charge_rescue"));
        assert_eq!(
            decide(&failed, attempt(1), &charge),
            Decision::Rescue(rescue, cause)
        );
    }

    #[rstest]
    #[case::a_failure(declined(), StepFailureCause::Failure)]
    #[case::an_abnormal_termination(crashed(), StepFailureCause::AbnormalTermination)]
    fn a_rescued_step_is_given_up_after_the_attempt_that_follows_its_rescue(
        #[case] failed: Failed,
        #[case] cause: StepFailureCause,
    ) {
        assert_eq!(
            decide(&failed, attempt(2), &rescued_charge()),
            Decision::GiveUp(cause)
        );
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
