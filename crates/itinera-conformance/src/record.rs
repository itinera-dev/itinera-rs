//! The reporters that record what a journey emits, and the dispatchers that hold them.

mod dispatcher;
mod recorders;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use itinera::error::Error;
use itinera::event::{Event, EventKind};
use itinera::report::{GlobalMetadata, Reporter};
use itinera::value::AnyValue;

pub(crate) use dispatcher::{Behaviour, RecordingFactory};
pub(crate) use recorders::{NoStream, Recorders};

use crate::model::{Model, ReporterFailure};

/// A scripted reporter that records every event and entry of global metadata it receives, then
/// fails as its script says.
///
/// Clones share what they record, so the scenario reads what the reporter that the executor
/// holds received.
#[derive(Clone, Debug, Default)]
pub(crate) struct Recorder {
    shared: Arc<Mutex<Received>>,
}

#[derive(Debug, Default)]
struct Received {
    events: Vec<Event>,
    entries: Vec<Entry>,
    failure: Option<ReporterFailure>,
    failed: bool,
}

/// An entry of global metadata a reporter received, including one it failed on.
#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) key: String,
    pub(crate) value: AnyValue,
    /// The kind of the last event the reporter received before it, if any.
    pub(crate) after: Option<EventKind>,
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

    /// The journey ID the reporter received, as global metadata before any event, if it
    /// received one.
    pub(crate) fn journey_id(&self) -> Option<String> {
        self.received()
            .entries
            .iter()
            .find(is_the_journey_id)
            .and_then(text)
            .cloned()
    }

    /// The entries of global metadata it received, in order, including those it failed on.
    pub(crate) fn entries(&self) -> Vec<Entry> {
        self.received().entries.clone()
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
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        let mut received = self.received();
        let after = received.events.last().map(Event::kind);
        received.entries.push(Entry {
            key: key.to_owned(),
            value: value.clone(),
            after,
        });
        if received.failure != Some(ReporterFailure::OnGlobalMetadata(key.to_owned())) {
            return Ok(());
        }
        received.failed = true;
        Err(Error::msg(format!(
            "the scripted reporter fails on the global metadata {key}"
        )))
    }

    fn report(&mut self, event: &Event) -> Result<(), Error> {
        let mut received = self.received();
        received.events.push(event.clone());
        let fails = match &received.failure {
            None => None,
            Some(ReporterFailure::First) => (received.events.len() == 1).then_some(None),
            Some(ReporterFailure::On(kind, message)) => kind.is_of(event).then(|| message.clone()),
            Some(ReporterFailure::OnGlobalMetadata(_)) => None,
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

/// Whether the entry is the journey ID, received before any event.
fn is_the_journey_id(entry: &&Entry) -> bool {
    entry.key == GlobalMetadata::JOURNEY_ID && entry.after.is_none()
}

/// The entry's value, if it is a string.
fn text(entry: &Entry) -> Option<&String> {
    entry.value.downcast_ref()
}

/// What a scripted reporter fails with when its script gives no message.
fn failure(event: &Event) -> String {
    format!("the scripted reporter fails on {}", event.kind())
}
