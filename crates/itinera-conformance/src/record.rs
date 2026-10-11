//! The reporters that record what a journey emits, and the dispatchers that hold them.

mod dispatcher;
mod recorders;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use itinera::error::Error;
use itinera::event::Event;
use itinera::journey::JourneyId;
use itinera::report::Reporter;

pub(crate) use dispatcher::{Behaviour, RecordingFactory};
pub(crate) use recorders::{NoStream, Recorders};

use crate::model::{Model, ReporterFailure};

/// A scripted reporter that records every event it receives, then fails as its script says.
///
/// Clones share what they record, so the scenario reads what the reporter that the executor
/// holds received.
#[derive(Clone, Debug, Default)]
pub(crate) struct Recorder {
    shared: Arc<Mutex<Received>>,
}

#[derive(Debug, Default)]
struct Received {
    /// The journey ID the reporter was given as its workflow instance was created.
    journey_id: Option<JourneyId>,
    events: Vec<Event>,
    failure: Option<ReporterFailure>,
    failed: bool,
}

impl Recorder {
    pub(crate) fn failing(failure: Option<ReporterFailure>) -> Self {
        Self {
            shared: Arc::new(Mutex::new(Received {
                failure,
                ..Received::default()
            })),
        }
    }

    /// A recorder that fails as the scenario scripts the reporter of this name.
    fn scripted(model: &Model, name: &str) -> Self {
        Self::failing(model.reporter_failures.get(name).cloned())
    }

    fn received(&self) -> MutexGuard<'_, Received> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The journey ID the reporter was given, if a workflow instance gave it one.
    pub(crate) fn journey_id(&self) -> Option<JourneyId> {
        self.received().journey_id.clone()
    }

    /// The events it received, in order, including those it failed on.
    pub(crate) fn events(&self) -> Vec<Event> {
        self.received().events.clone()
    }

    /// Whether it never failed, so that it received every event dispatched to it.
    pub(crate) fn never_failed(&self) -> bool {
        !self.received().failed
    }
}

impl Reporter for Recorder {
    fn set_journey_id(&mut self, journey_id: &JourneyId) -> Result<(), Error> {
        self.received().journey_id = Some(journey_id.clone());
        Ok(())
    }

    fn report(&mut self, event: &Event) -> Result<(), Error> {
        let mut received = self.received();
        received.events.push(event.clone());
        let fails = match &received.failure {
            None => None,
            Some(ReporterFailure::First) => (received.events.len() == 1).then_some(None),
            Some(ReporterFailure::On(kind, message)) => kind.is_of(event).then(|| message.clone()),
        };
        match fails {
            None => Ok(()),
            Some(message) => {
                received.failed = true;
                Err(Error::msg(message.unwrap_or_else(|| failure(event))))
            }
        }
    }
}

/// What a scripted reporter fails with when its script gives no message.
fn failure(event: &Event) -> String {
    format!("the scripted reporter fails on {}", event.kind())
}
