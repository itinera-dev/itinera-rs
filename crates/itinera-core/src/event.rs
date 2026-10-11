//! Events: the record of what happens in a journey, and what only events carry.

use std::num::NonZeroU64;

use crate::journey::JourneyId;
use crate::workflow::WorkflowName;

mod abort;
mod body;
mod decision;
mod failure;
#[cfg(test)]
pub(crate) mod fixtures;
mod requester;
mod source;
mod timestamp;

pub use abort::{JourneyAbort, MissingData};
pub use body::EventBody;
pub use decision::{DecidingHook, GiveUpCause, GiveUpHook};
pub use failure::JourneyFailure;
pub use requester::Requester;
pub use source::{HookSource, RequestSource, Source};
pub use timestamp::Timestamp;

/// One event of a journey's event stream.
///
/// Events are made only by itinera's executors. Every event carries a sequence number,
/// increasing from 1 within the journey, a timestamp, the journey ID and the workflow name;
/// what else it carries depends on its [`body`](Event::body).
///
/// Events carry no format of their own: each reporter writes them as it chooses.
///
/// # Examples
///
/// ```
/// use itinera::event::{Event, EventBody};
///
/// fn summary(event: &Event) -> String {
///     match &event.body {
///         EventBody::StepFailed { step, reason, .. } => {
///             format!("{} failed: {}", step.step, reason.code())
///         }
///         _ => format!("#{} {}", event.sequence, event.kind()),
///     }
/// }
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Event {
    /// The event's position in the journey's event stream, from 1.
    pub sequence: NonZeroU64,
    /// When the event was emitted.
    pub timestamp: Timestamp,
    /// The journey's ID.
    pub journey_id: JourneyId,
    /// The workflow's name.
    pub workflow: WorkflowName,
    /// What the event says.
    pub body: EventBody,
}
impl Event {
    /// The event's kind, in snake_case, for example `step_failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::Event;
    ///
    /// fn is_last(event: &Event) -> bool {
    ///     matches!(event.kind(), "journey_succeeded" | "journey_failed" | "journey_aborted")
    /// }
    /// ```
    pub fn kind(&self) -> &'static str {
        self.body.kind()
    }
}
