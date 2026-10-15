//! Fixtures the tests of several modules share: reporters that record what they are given, and
//! the events they are given.

use std::sync::{Arc, Mutex};

use crate::error::Error;
use crate::event::fixtures::event;
use crate::event::{Event, EventBody, JourneyAbort};
use crate::report::Reporter;
use crate::value::AnyValue;

/// What the journey's reporters received, in order, as "reporter kind" lines.
pub(crate) type Log = Arc<Mutex<Vec<String>>>;

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

    /// Logs what it received, the kind of an event or the key of an entry of global metadata,
    /// unless it fails on it.
    pub(super) fn record(&mut self, received: &str) -> Result<(), Error> {
        if self.fails_on == Some(received) {
            return Err(Error::msg(format!("{} failed", self.name)));
        }
        self.log
            .lock()
            .unwrap()
            .push(format!("{} {received}", self.name));
        Ok(())
    }
}

impl Reporter for Recording {
    fn set_global_metadata(&mut self, key: &str, _value: &AnyValue) -> Result<(), Error> {
        self.record(key)
    }

    fn report(&mut self, event: &Event) -> Result<(), Error> {
        self.record(&event.kind().to_string())
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

pub(crate) fn entries(log: &Log) -> Vec<String> {
    log.lock().unwrap().clone()
}
