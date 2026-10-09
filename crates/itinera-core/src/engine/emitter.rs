use std::num::NonZeroU64;
use std::time::SystemTime;

use crate::event::{Event, EventBody, Timestamp};
use crate::journey::JourneyId;
use crate::workflow::WorkflowName;

/// Where the emitter reads the time: the system's clock, or a test's.
pub(crate) type Clock = fn() -> SystemTime;

/// Makes the journey's events, numbering them from 1 and stamping them with the time.
pub(crate) struct Emitter {
    journey_id: JourneyId,
    workflow: WorkflowName,
    next: NonZeroU64,
    clock: Clock,
}

impl Emitter {
    pub(crate) fn new(journey_id: JourneyId, workflow: WorkflowName, clock: Clock) -> Self {
        Self {
            journey_id,
            workflow,
            next: NonZeroU64::MIN,
            clock,
        }
    }

    pub(crate) fn next(&mut self, body: EventBody) -> Event {
        let sequence = self.next;
        self.next = sequence.saturating_add(1);
        Event {
            sequence,
            timestamp: Timestamp::from((self.clock)()),
            journey_id: self.journey_id.clone(),
            workflow: self.workflow,
            body,
        }
    }
}
