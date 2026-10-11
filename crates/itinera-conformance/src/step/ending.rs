//! How each attempt of a scripted step ends, and what it contributes just before.

use itinera::error::Error;
use itinera::step::{Outcome, Reason};
use itinera::value::Value as Storable;

use crate::model::{self, AttemptOutcome, ModelError};
use crate::value::{Takes, Typed};

/// How one attempt ends, and what it contributes just before.
#[derive(Debug)]
pub(super) struct Ending {
    pub(super) contributes: Vec<(String, Typed)>,
    pub(super) end: End,
}

#[derive(Debug)]
pub(super) enum End {
    Success,
    Failure(Reason),
    RetriableFailure(Reason),
    Skipped(Option<Reason>),
    /// An error escapes the step, with this message.
    Error(String),
}

impl Ending {
    pub(super) fn success() -> Self {
        Self {
            contributes: Vec::new(),
            end: End::Success,
        }
    }

    pub(super) fn of(attempt: &model::Attempt) -> Result<Self, ModelError> {
        Ok(Self {
            contributes: attempt
                .contributes
                .iter()
                .map(typed_entry)
                .collect::<Result<_, _>>()?,
            end: End::of(&attempt.outcome)?,
        })
    }
}

fn typed_entry((key, value): &(String, serde_json::Value)) -> Result<(String, Typed), ModelError> {
    Ok((key.clone(), Typed::of(value)?))
}

impl End {
    fn of(outcome: &AttemptOutcome) -> Result<Self, ModelError> {
        Ok(match outcome {
            AttemptOutcome::Success => Self::Success,
            AttemptOutcome::Failure {
                reason,
                retriable: false,
            } => Self::Failure(reason_of(reason)?),
            AttemptOutcome::Failure {
                reason,
                retriable: true,
            } => Self::RetriableFailure(reason_of(reason)?),
            AttemptOutcome::Skipped(reason) => {
                Self::Skipped(reason.as_ref().map(reason_of).transpose()?)
            }
            AttemptOutcome::Error(message) => Self::Error(
                message
                    .clone()
                    .unwrap_or_else(|| "the scripted step fails".to_owned()),
            ),
        })
    }

    pub(super) fn outcome(&self) -> Result<Outcome, Error> {
        match self {
            Self::Success => Ok(Outcome::success()),
            Self::Failure(reason) => Ok(Outcome::failure(reason.clone())),
            Self::RetriableFailure(reason) => Ok(Outcome::retriable_failure(reason.clone())),
            Self::Skipped(None) => Ok(Outcome::skipped()),
            Self::Skipped(Some(reason)) => Ok(Outcome::skipped_because(reason.clone())),
            Self::Error(message) => Err(Error::msg(message.clone())),
        }
    }
}

/// The reason, with its message and its details when the case gives them.
fn reason_of(reason: &model::Reason) -> Result<Reason, ModelError> {
    let coded = Reason::new(reason.code.clone());
    let explained = match &reason.message {
        Some(message) => coded.with_message(message.clone()),
        None => coded,
    };
    match &reason.details {
        Some(details) => Ok(Typed::of(details)?.given_to(Detailing { reason: explained })),
        None => Ok(explained),
    }
}

/// Gives a reason its details.
struct Detailing {
    reason: Reason,
}

impl Takes for Detailing {
    type Output = Reason;

    fn take<T: Storable>(self, details: T) -> Reason {
        self.reason.with_details(details)
    }
}
