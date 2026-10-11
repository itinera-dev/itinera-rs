use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::error::Error;
use crate::event::{Event, EventBody};
use crate::report::Reporter;

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
            Err(error) => {
                self.failed = true;
                let message = error.to_string();
                self.failures.fail(error);
                Err(Error::msg(message))
            }
            Ok(()) => Ok(()),
        }
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
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::executor::fixtures::{Execute, Shop, entries, on, run_on};
    use crate::journey::{Abort, JourneyStatus};
    use crate::report::{DefaultDispatcherFactory, Dispatcher, DispatcherFactory};

    #[rstest]
    #[case::the_default_dispatcher(on::<DefaultDispatcherFactory>)]
    #[case::a_dispatcher_that_ignores_failures(on::<CarelessFactory>)]
    #[case::a_dispatcher_that_stops_at_any_failure(on::<StrictFactory>)]
    fn a_reporter_that_fails_aborts_the_journey_and_only_journey_aborted_reaches_the_others(
        #[case] execute: Execute,
    ) {
        let (result, log) = run_on(execute, Shop::failing("fragile", "attempt_started"));

        let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = result.status else {
            panic!(
                "the journey was not aborted by a reporter: {:?}",
                result.status
            );
        };
        assert_eq!(error.to_string(), "fragile failed on attempt_started");
        assert_eq!(
            entries(&log),
            [
                "audit journey_started",
                "fragile journey_started",
                "metrics journey_started",
                "audit attempt_started",
                "fragile attempt_started",
                "audit journey_aborted",
                "metrics journey_aborted",
            ]
        );
    }

    #[rstest]
    #[case::the_default_dispatcher(on::<DefaultDispatcherFactory>)]
    #[case::a_dispatcher_that_ignores_failures(on::<CarelessFactory>)]
    #[case::a_dispatcher_that_stops_at_any_failure(on::<StrictFactory>)]
    fn a_reporter_that_fails_while_journey_aborted_is_delivered_is_ignored(
        #[case] execute: Execute,
    ) {
        let mut shop = Shop::failing("audit", "attempt_started");
        shop.fails_on.push(("fragile", "journey_aborted"));
        let (result, log) = run_on(execute, shop);

        let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = result.status else {
            panic!(
                "the journey was not aborted by a reporter: {:?}",
                result.status
            );
        };
        assert_eq!(error.to_string(), "audit failed on attempt_started");
        assert_eq!(
            entries(&log),
            [
                "audit journey_started",
                "fragile journey_started",
                "metrics journey_started",
                "audit attempt_started",
                "fragile journey_aborted",
                "metrics journey_aborted",
            ]
        );
    }

    /// A dispatcher that ignores the errors of its reporters and delivers every event to all.
    #[derive(Default)]
    struct Careless {
        reporters: Vec<Box<dyn Reporter>>,
    }

    impl Dispatcher for Careless {
        fn add(&mut self, reporter: Box<dyn Reporter>) -> Result<(), Error> {
            self.reporters.push(reporter);
            Ok(())
        }

        fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
            for reporter in &mut self.reporters {
                let _ignored = reporter.report(event);
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct CarelessFactory;

    impl DispatcherFactory for CarelessFactory {
        type Dispatcher = Careless;

        fn create(&mut self) -> Result<Careless, Error> {
            Ok(Careless::default())
        }
    }

    /// A dispatcher that stops delivering an event at the first reporter that fails, whatever
    /// the event.
    #[derive(Default)]
    struct Strict {
        reporters: Vec<Box<dyn Reporter>>,
    }

    impl Dispatcher for Strict {
        fn add(&mut self, reporter: Box<dyn Reporter>) -> Result<(), Error> {
            self.reporters.push(reporter);
            Ok(())
        }

        fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
            self.reporters
                .iter_mut()
                .try_for_each(|reporter| reporter.report(event))
        }
    }

    #[derive(Default)]
    struct StrictFactory;

    impl DispatcherFactory for StrictFactory {
        type Dispatcher = Strict;

        fn create(&mut self) -> Result<Strict, Error> {
            Ok(Strict::default())
        }
    }
}
