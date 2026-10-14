//! How each attempt of a scripted step ends, and what it contributes just before.

use itinera::step::{Ending, Reason};
use itinera::value::Value as Storable;

use crate::model::{self, AttemptOutcome, ModelError};
use crate::value::{Takes, Typed};

/// How one attempt ends, and what it contributes just before.
#[derive(Debug)]
pub(super) struct AttemptEnd {
    pub(super) contributes: Vec<(String, Typed)>,
    pub(super) ending: Ending,
}

impl AttemptEnd {
    pub(super) fn success() -> Self {
        Self {
            contributes: Vec::new(),
            ending: Ending::Success,
        }
    }

    pub(super) fn skips(&self) -> bool {
        matches!(self.ending, Ending::Skipped(_))
    }

    pub(super) fn of(attempt: &model::Attempt) -> Result<Self, ModelError> {
        Ok(Self {
            contributes: attempt
                .contributes
                .iter()
                .map(typed_entry)
                .collect::<Result<_, _>>()?,
            ending: ending_of(&attempt.outcome)?,
        })
    }
}

fn typed_entry((key, value): &(String, serde_json::Value)) -> Result<(String, Typed), ModelError> {
    Ok((key.clone(), Typed::of(value)?))
}

/// How an attempt ends, as its row says. An error with no message given carries one saying
/// the scripted step fails.
fn ending_of(outcome: &AttemptOutcome) -> Result<Ending, ModelError> {
    Ok(match outcome {
        AttemptOutcome::Success => Ending::Success,
        AttemptOutcome::Failure(reason) => Ending::Failure(reason_of(reason)?),
        AttemptOutcome::Skipped(reason) => {
            Ending::Skipped(reason.as_ref().map(reason_of).transpose()?)
        }
        AttemptOutcome::Error(message) => Ending::AbnormalTermination(
            message
                .clone()
                .unwrap_or_else(|| "the scripted step fails".to_owned()),
        ),
    })
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
