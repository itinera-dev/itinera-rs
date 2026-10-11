//! Fixtures the tests of several modules share: an event of a journey.

use std::num::NonZeroU64;
use std::time::{Duration, UNIX_EPOCH};

use super::{Event, EventBody, Timestamp};
use crate::journey::JourneyId;
use crate::workflow::WorkflowName;

pub(crate) fn event(sequence: u64, body: EventBody) -> Event {
    Event {
        sequence: NonZeroU64::new(sequence).unwrap(),
        timestamp: Timestamp::from(UNIX_EPOCH + Duration::from_millis(1_700_000_000_123)),
        journey_id: JourneyId::from("order-42"),
        workflow: WorkflowName::from("orders"),
        body,
    }
}
