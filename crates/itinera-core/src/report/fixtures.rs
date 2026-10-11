//! Fixtures the tests of several modules share: reporters that record what they are given, and
//! the events they are given.

use std::sync::{Arc, Mutex};

use crate::error::Error;
use crate::event::fixtures::event;
use crate::event::{Event, EventBody, JourneyAbort};
use crate::report::Reporter;

pub(super) type Log = Arc<Mutex<Vec<String>>>;

pub(super) struct Recording {
    name: &'static str,
    log: Log,
    fails_on: Option<&'static str>,
}

impl Recording {
    pub(super) fn new(name: &'static str, log: &Log) -> Self {
        Self {
            name,
            log: Arc::clone(log),
            fails_on: None,
        }
    }

    pub(super) fn failing_on(name: &'static str, kind: &'static str, log: &Log) -> Self {
        Self {
            fails_on: Some(kind),
            ..Self::new(name, log)
        }
    }

    pub(super) fn record(&mut self, event: &Event) -> Result<(), Error> {
        if self.fails_on == Some(event.kind()) {
            return Err(Error::msg(format!("{} failed", self.name)));
        }
        self.log
            .lock()
            .unwrap()
            .push(format!("{} {}", self.name, event.kind()));
        Ok(())
    }
}

impl Reporter for Recording {
    fn report(&mut self, event: &Event) -> Result<(), Error> {
        self.record(event)
    }
}

pub(super) fn started() -> Event {
    event(
        1,
        EventBody::JourneyStarted {
            initial_keys: Vec::new(),
        },
    )
}

pub(super) fn aborted() -> Event {
    event(
        2,
        EventBody::JourneyAborted {
            abort: JourneyAbort::ReporterFailed {
                step: None,
                error: "a failed".to_string(),
            },
        },
    )
}

pub(super) fn entries(log: &Log) -> Vec<String> {
    log.lock().unwrap().clone()
}
