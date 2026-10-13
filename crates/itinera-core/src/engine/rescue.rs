//! Rescuing a step: what its failed attempt leaves for its rescue step, the decisions
//! `step_rescuing` and `step_rescued`, and running the rescue step before the step is attempted
//! again.

use super::attempt::Attempting;
use super::decision::Failed;
use super::end::End;
use super::request::Requesting;
use super::{Delivery, Journey, Next};
use crate::error::Error;
use crate::event::{self, EventBody, JourneyAbort, RescueCause};
use crate::instance::WorkflowInstance;
use crate::journey::{Abort, Contributed, FailureRequester, MissingData};
use crate::step::{FromRescued, Reason, Requirement, RescueRequest, StepAttempt, StepDescriptor};
use crate::value::AnyValue;

#[cfg(test)]
mod tests;

/// What the failed attempt of a step leaves for its rescue step to request: what it contributed,
/// never committed, and the failure's reason or the error.
#[derive(Debug)]
pub(super) struct LeftForRescue {
    contributed: Contributed,
    reason: Option<Reason>,
    error: Option<Error>,
}

impl LeftForRescue {
    fn new(contributed: Contributed, failed: Failed) -> Self {
        let (reason, error) = match failed {
            Failed::Failure(reason) | Failed::RetriableFailure(reason) => (Some(reason), None),
            Failed::AbnormalTermination(error) => (None, Some(error)),
        };
        Self {
            contributed,
            reason,
            error,
        }
    }

    /// What the failed attempt contributed under the key, if anything.
    fn contributed(&self, key: &str) -> Option<AnyValue> {
        self.contributed.get(key).cloned()
    }

    fn take_reason(&mut self) -> Option<Reason> {
        self.reason.take()
    }

    fn take_error(&mut self) -> Option<Error> {
        self.error.take()
    }
}

impl<D: Delivery> Journey<D> {
    /// Hands an attempt that did not succeed to the step's rescue step: records `step_rescuing`,
    /// and keeps what the attempt left for the rescue step.
    pub(super) async fn rescuing<'s, W, M>(
        &mut self,
        attempting: &Attempting<'s, W, M>,
        rescue: &'s StepDescriptor<W, M>,
        failed: Failed,
        cause: RescueCause,
    ) -> Result<Next<'s, W, M>, End> {
        self.emit(EventBody::StepRescuing {
            step: attempting.attempt.clone(),
            cause,
            rescue: rescue.name(),
        })
        .await?;
        let rescuing = LeftForRescue::new(attempting.contributed.clone(), failed);
        Ok(Next::Rescue(rescue, Box::new(rescuing)))
    }

    /// Runs the rescue step of a step whose attempt failed, as its own attempt 1, then records
    /// `step_rescued` once it succeeded. A rescue step that does not succeed ends the journey.
    pub(super) async fn rescue<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        rescue: &StepDescriptor<I::Workflow, I::Mode>,
        failed: &StepAttempt,
        rescuing: LeftForRescue,
    ) -> Result<(), End> {
        self.step = Some(rescue.name());
        let first = StepAttempt::first(rescue.name());
        // A rescue step has no policies, no retry budget and no rescue step of its own, so an
        // attempt of it that does not end the journey succeeded.
        self.attempt(instance, rescue, &first, Some(rescuing))
            .await?;
        self.step = Some(failed.step);
        self.emit(EventBody::StepRescued {
            step: failed.clone(),
            rescue: rescue.name(),
        })
        .await?;
        Ok(())
    }

    /// Resolves one of a rescue step's requests to the step it rescues, from what its failed
    /// attempt left, into what the rescue step receives.
    pub(super) async fn rescue_request(
        &mut self,
        got: &mut FromRescued,
        attempt: &StepAttempt,
        request: &RescueRequest,
        left: Option<&mut LeftForRescue>,
    ) -> Result<(), End> {
        match request {
            RescueRequest::FromRescuedStep(need) => {
                let key = need.key();
                let found = left.and_then(|left| left.contributed(key));
                let value = self
                    .request(&Requesting::step(attempt), need, found)
                    .await?;
                got.contributed.hold(key, value);
            }
            RescueRequest::Reason(requirement) => {
                let found = left.and_then(LeftForRescue::take_reason);
                got.reason = Some(required(*requirement, found, attempt, missing_reason)?);
            }
            RescueRequest::Error(requirement) => {
                let found = left.and_then(LeftForRescue::take_error);
                got.error = Some(required(*requirement, found, attempt, missing_error)?);
            }
        }
        Ok(())
    }
}

/// What a rescue step receives for its request of the failure's reason or the error: the value
/// found, or the abort a required request without one causes, naming the rescue step.
fn required<T>(
    requirement: Requirement,
    found: Option<T>,
    attempt: &StepAttempt,
    missing: fn(&StepAttempt) -> End,
) -> Result<Option<T>, End> {
    match (requirement, found) {
        (Requirement::Required, None) => Err(missing(attempt)),
        (_, found) => Ok(found),
    }
}

fn missing_reason(attempt: &StepAttempt) -> End {
    End::aborted(
        Abort::RequiredDataMissing(MissingData::Reason {
            requester: FailureRequester::RescueStep,
        }),
        JourneyAbort::RequiredDataMissing {
            missing: event::MissingData::Reason {
                requester: event::FailureRequester::RescueStep { step: attempt.step },
            },
        },
    )
}

fn missing_error(attempt: &StepAttempt) -> End {
    End::aborted(
        Abort::RequiredDataMissing(MissingData::Error {
            requester: FailureRequester::RescueStep,
        }),
        JourneyAbort::RequiredDataMissing {
            missing: event::MissingData::Error {
                requester: event::FailureRequester::RescueStep { step: attempt.step },
            },
        },
    )
}
