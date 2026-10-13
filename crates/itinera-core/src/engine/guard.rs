use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::error::Error;
use crate::event::{Event, EventBody};
use crate::report::Reporter;
use crate::value::AnyValue;

/// What the reporters of one journey share: whether one of them has failed, and the error of
/// the first that did, until the engine takes it.
///
/// Each reporter the executor adds to the dispatcher is guarded, so that whatever the
/// dispatcher does, once a reporter has failed the others receive only `journey_aborted`, and the
/// one that failed receives nothing more.
#[derive(Clone, Debug, Default)]
pub(crate) struct Failures {
    shared: Arc<Mutex<Shared>>,
}

#[derive(Debug, Default)]
enum Shared {
    /// No reporter has failed.
    #[default]
    Running,
    /// A reporter has failed, with this error until the engine takes it.
    Failed { error: Option<Error> },
}

impl Failures {
    fn shared(&self) -> MutexGuard<'_, Shared> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The error of the reporter that failed, the first time it is asked for.
    pub(crate) fn take(&self) -> Option<Error> {
        match &mut *self.shared() {
            Shared::Failed { error } => error.take(),
            Shared::Running => None,
        }
    }

    fn running(&self) -> bool {
        matches!(*self.shared(), Shared::Running)
    }

    /// Records that a reporter failed, keeping the error of the first that did.
    fn fail(&self, error: Error) {
        let mut shared = self.shared();
        if let Shared::Running = *shared {
            *shared = Shared::Failed { error: Some(error) };
        }
    }

    /// Guards a synchronous reporter.
    pub(crate) fn guard(&self, reporter: Box<dyn Reporter>) -> Box<dyn Reporter> {
        Box::new(Guarded {
            reporter,
            guard: Guard::new(self.clone()),
        })
    }
}

/// What one guarded reporter knows: the journey's failures, and whether it failed itself.
#[derive(Debug)]
pub(crate) struct Guard {
    failures: Failures,
    failed: bool,
}

impl Guard {
    pub(crate) fn new(failures: Failures) -> Self {
        Self {
            failures,
            failed: false,
        }
    }

    /// Whether the reporter receives the event: never once it has failed, and once another has
    /// failed, only `journey_aborted`.
    pub(crate) fn admits(&self, event: &Event) -> bool {
        !self.failed && (is_aborted(event) || self.failures.running())
    }

    /// Records what reporting the event gave. A failure on any event but `journey_aborted`
    /// fails the journey: the engine gets the reporter's own error, and the dispatcher one with
    /// the same message, so that it stops delivering the event. A failure on `journey_aborted`
    /// is ignored, and the dispatcher never sees it, so that it delivers to the rest.
    pub(crate) fn record(
        &mut self,
        event: &Event,
        reported: Result<(), Error>,
    ) -> Result<(), Error> {
        match reported {
            Err(_) if is_aborted(event) => Ok(()),
            reported => self.fail_on_error(reported),
        }
    }

    /// Whether the reporter receives an entry of global metadata: only while no reporter has
    /// failed.
    pub(crate) fn admits_metadata(&self) -> bool {
        !self.failed && self.failures.running()
    }

    /// Records what a reporter's call gave, receiving an entry of global metadata or any event
    /// but `journey_aborted`: a failure fails the journey.
    pub(crate) fn fail_on_error(&mut self, reported: Result<(), Error>) -> Result<(), Error> {
        reported.map_err(|error| self.fail(error))
    }

    fn fail(&mut self, error: Error) -> Error {
        self.failed = true;
        let message = error.to_string();
        self.failures.fail(error);
        Error::msg(message)
    }
}

fn is_aborted(event: &Event) -> bool {
    matches!(event.body, EventBody::JourneyAborted { .. })
}

struct Guarded {
    reporter: Box<dyn Reporter>,
    guard: Guard,
}

impl Reporter for Guarded {
    fn report(&mut self, event: &Event) -> Result<(), Error> {
        if !self.guard.admits(event) {
            return Ok(());
        }
        let reported = self.reporter.report(event);
        self.guard.record(event, reported)
    }

    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        if !self.guard.admits_metadata() {
            return Ok(());
        }
        let reported = self.reporter.set_global_metadata(key, value);
        self.guard.fail_on_error(reported)
    }
}

#[cfg(test)]
mod tests;
