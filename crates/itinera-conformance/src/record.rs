//! The reporters that record what a journey emits, and the dispatchers that hold them.

mod dispatcher;
mod recorders;

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use itinera::error::Error;
use itinera::event::Event;
use itinera::journey::{DataBag, JourneyId};
use itinera::report::{Reporter, WorkflowReporter};

pub(crate) use dispatcher::{Behaviour, RecordingFactory};
pub(crate) use recorders::{NoStream, Recorders};

use crate::declaration::ScriptedWorkflow;
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
    /// The journey ID a workflow instance made the reporter with.
    made_with: Option<JourneyId>,
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

    /// Records the journey ID the workflow instance made the reporter with.
    fn made_with(&self, journey_id: &JourneyId) {
        self.received().made_with = Some(journey_id.clone());
    }

    /// The journey ID a workflow instance made the reporter with, if one made it.
    pub(crate) fn journey_id(&self) -> Option<JourneyId> {
        self.received().made_with.clone()
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

/// The reporter the workflow lists at this position, which records with the recorder the
/// instance was given for it.
#[derive(Debug)]
pub(crate) struct ListedReporter<const POSITION: usize> {
    recorder: Recorder,
}

impl<const POSITION: usize> Reporter for ListedReporter<POSITION> {
    fn report(&mut self, event: &Event) -> Result<(), Error> {
        self.recorder.report(event)
    }
}

impl<const POSITION: usize> WorkflowReporter<ScriptedWorkflow> for ListedReporter<POSITION> {
    fn init(
        workflow: &ScriptedWorkflow,
        journey_id: &JourneyId,
        _: &DataBag,
    ) -> Result<Self, Error> {
        let recorder = workflow
            .reporter(POSITION)
            .ok_or_else(|| {
                Error::msg("the runner gave the instance no recorder for this reporter")
            })?
            .clone();
        recorder.made_with(journey_id);
        Ok(Self { recorder })
    }
}

/// What a scripted reporter fails with when its script gives no message.
fn failure(event: &Event) -> String {
    format!("the scripted reporter fails on {}", event.kind())
}
