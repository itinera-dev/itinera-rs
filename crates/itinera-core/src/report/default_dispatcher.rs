//! The default dispatcher, which calls its reporters in order, and its factory.

use super::{Dispatcher, DispatcherFactory, Reporter, sealed};
use crate::error::Error;
use crate::event::{Event, EventBody};

/// The dispatcher executors use unless they are given another factory.
///
/// It calls its reporters in the order they were added. When a reporter fails, the event stops
/// there and the error is returned, except for `journey_aborted`, whose delivery goes on past a
/// failure, returning the first error. The executor, not the dispatcher, keeps `journey_aborted`
/// from the reporter that failed.
///
/// `DefaultDispatcher` holds synchronous reporters. With the `async` feature,
/// `DefaultDispatcher<BoxedReporter>` is the asynchronous dispatcher, which holds both kinds.
/// It can hold nothing else:
///
/// ```compile_fail,E0277
/// use itinera::report::DefaultDispatcher;
///
/// let dispatcher = DefaultDispatcher::<u8>::default();
/// ```
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::report::{DefaultDispatcher, Dispatcher, Reporter};
///
/// struct Silent;
///
/// impl Reporter for Silent {
///     fn report(&mut self, _event: &Event) -> Result<(), Error> {
///         Ok(())
///     }
/// }
///
/// let mut dispatcher = DefaultDispatcher::new();
/// dispatcher.add(Box::new(Silent))?;
/// # Ok::<(), Error>(())
/// ```
#[derive(derive_more::Debug)]
pub struct DefaultDispatcher<R: sealed::Held = Box<dyn Reporter>> {
    #[debug("{}", reporters.len())]
    pub(super) reporters: Vec<R>,
}

impl DefaultDispatcher {
    /// Makes a dispatcher holding no reporter.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::report::DefaultDispatcher;
    ///
    /// let dispatcher = DefaultDispatcher::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

impl<R: sealed::Held> Default for DefaultDispatcher<R> {
    fn default() -> Self {
        Self {
            reporters: Vec::new(),
        }
    }
}

impl Dispatcher for DefaultDispatcher {
    fn add(&mut self, reporter: Box<dyn Reporter>) -> Result<(), Error> {
        self.reporters.push(reporter);
        Ok(())
    }

    fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
        let mut delivery = Delivery::of(event);
        for reporter in &mut self.reporters {
            if delivery.failed(reporter.report(event)) {
                break;
            }
        }
        delivery.finish()
    }
}

/// The factory of [`DefaultDispatcher`], which executors use unless they are given another.
///
/// # Examples
///
/// ```
/// use itinera::report::{DefaultDispatcherFactory, DispatcherFactory};
///
/// let mut factory = DefaultDispatcherFactory;
/// let dispatcher = factory.create();
/// assert!(dispatcher.is_ok());
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultDispatcherFactory;

impl DispatcherFactory for DefaultDispatcherFactory {
    type Dispatcher = DefaultDispatcher;

    fn create(&mut self) -> Result<DefaultDispatcher, Error> {
        Ok(DefaultDispatcher::new())
    }
}

pub(super) struct Delivery {
    to_everyone: bool,
    error: Option<Error>,
}

impl Delivery {
    pub(super) fn of(event: &Event) -> Self {
        Self {
            to_everyone: matches!(event.body, EventBody::JourneyAborted { .. }),
            error: None,
        }
    }

    pub(super) fn failed(&mut self, result: Result<(), Error>) -> bool {
        if let Err(error) = result {
            self.error.get_or_insert(error);
        }
        self.error.is_some() && !self.to_everyone
    }

    pub(super) fn finish(self) -> Result<(), Error> {
        self.error.map_or(Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::fixtures::event;
    use crate::report::fixtures::{Log, Recording, aborted, entries, started};

    fn succeeded() -> Event {
        event(2, EventBody::JourneySucceeded { decided_by: None })
    }

    #[test]
    fn the_default_dispatcher_calls_reporters_in_the_order_they_were_added() {
        let log = Log::default();
        let mut dispatcher = DispatcherFactory::create(&mut DefaultDispatcherFactory).unwrap();
        dispatcher
            .add(Box::new(Recording::new("audit", &log)))
            .unwrap();
        dispatcher
            .add(Box::new(Recording::new("metrics", &log)))
            .unwrap();

        dispatcher.dispatch(&started()).unwrap();
        dispatcher.dispatch(&succeeded()).unwrap();

        assert_eq!(
            entries(&log),
            [
                "audit journey_started",
                "metrics journey_started",
                "audit journey_succeeded",
                "metrics journey_succeeded",
            ]
        );
    }

    #[test]
    fn delivery_of_an_event_stops_at_the_reporter_that_failed() {
        let log = Log::default();
        let mut dispatcher = DefaultDispatcher::new();
        dispatcher
            .add(Box::new(Recording::new("audit", &log)))
            .unwrap();
        dispatcher
            .add(Box::new(Recording::failing_on(
                "fragile",
                "journey_started",
                &log,
            )))
            .unwrap();
        dispatcher
            .add(Box::new(Recording::new("metrics", &log)))
            .unwrap();

        let error = dispatcher.dispatch(&started()).unwrap_err();

        assert_eq!(error.to_string(), "fragile failed");
        assert_eq!(entries(&log), ["audit journey_started"]);
    }

    #[test]
    fn a_failure_while_journey_aborted_is_delivered_does_not_stop_its_delivery() {
        let log = Log::default();
        let mut dispatcher = DefaultDispatcher::new();
        dispatcher
            .add(Box::new(Recording::failing_on(
                "fragile",
                "journey_aborted",
                &log,
            )))
            .unwrap();
        dispatcher
            .add(Box::new(Recording::failing_on(
                "broken",
                "journey_aborted",
                &log,
            )))
            .unwrap();
        dispatcher
            .add(Box::new(Recording::new("audit", &log)))
            .unwrap();

        let error = dispatcher.dispatch(&aborted()).unwrap_err();

        assert_eq!(error.to_string(), "fragile failed");
        assert_eq!(entries(&log), ["audit journey_aborted"]);
    }

    #[test]
    fn with_no_reporter_dispatching_succeeds_and_goes_nowhere() {
        let mut dispatcher = DefaultDispatcher::new();
        assert!(dispatcher.dispatch(&started()).is_ok());
    }
}
