//! Reporters, which receive the events of a journey, and dispatchers, which deliver them.

use std::fmt;

use crate::error::Error;
use crate::event::{Event, EventBody};
use crate::journey::{DataBag, JourneyId};

#[cfg(feature = "async")]
mod asynchronous;
mod sealed;

#[cfg(feature = "async")]
pub use asynchronous::{
    AsyncDispatcher, AsyncDispatcherFactory, AsyncReporter, AsyncWorkflowReporter, BoxedReporter,
};

/// Receives the events of a journey, and decides on its own what to do with each: filter it,
/// map it, forward it or ignore it.
///
/// A workflow lists its reporters, and each journey gets its own.
///
/// A reporter should never fail. One that cannot report an event, for example because the
/// service it forwards to is unavailable, should handle that itself (log it, drop the event,
/// keep it to retry later) and return `Ok`. An `Err` aborts the journey with `reporter failed`,
/// and the event it failed on never reaches the reporters after it, so the order of reporters
/// matters. `journey_aborted` then reaches every reporter except the one that failed. An `Err`
/// while `journey_aborted` itself is reported is ignored.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::report::Reporter;
///
/// #[derive(Default)]
/// struct Kinds {
///     kinds: Vec<&'static str>,
/// }
///
/// impl Reporter for Kinds {
///     fn report(&mut self, event: &Event) -> Result<(), Error> {
///         self.kinds.push(event.kind());
///         Ok(())
///     }
/// }
///
/// let reporter: Box<dyn Reporter> = Box::new(Kinds::default());
/// # drop(reporter);
/// ```
pub trait Reporter: Send + 'static {
    /// Receives one event.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::Reporter;
    ///
    /// struct Silent;
    ///
    /// impl Reporter for Silent {
    ///     fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn report(&mut self, event: &Event) -> Result<(), Error>;
}

/// A [`Reporter`] that a workflow lists, which every instance of the workflow makes for itself.
///
/// The workflow lists its reporters by type. Creating an instance makes each of them with
/// [`init`](WorkflowReporter::init), after the journey ID, so a reporter can be made for that
/// journey alone.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::journey::{DataBag, JourneyId};
/// use itinera::report::{Reporter, WorkflowReporter};
///
/// struct Orders;
///
/// struct Audit {
///     journey: JourneyId,
/// }
///
/// impl Reporter for Audit {
///     fn report(&mut self, event: &Event) -> Result<(), Error> {
///         eprintln!("{} {}", self.journey, event.kind());
///         Ok(())
///     }
/// }
///
/// impl WorkflowReporter<Orders> for Audit {
///     fn init(_: &Orders, journey_id: &JourneyId, _: &DataBag) -> Result<Self, Error> {
///         Ok(Audit {
///             journey: journey_id.clone(),
///         })
///     }
/// }
/// ```
pub trait WorkflowReporter<W>: Reporter + Sized {
    /// Makes the reporter for one journey, from the workflow's own value, the journey ID and the
    /// initial data. Failing makes creating the instance fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::journey::{DataBag, JourneyId};
    /// use itinera::report::{Reporter, WorkflowReporter};
    ///
    /// struct Orders {
    ///     audited: bool,
    /// }
    ///
    /// struct Audit;
    ///
    /// impl Reporter for Audit {
    ///     fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// impl WorkflowReporter<Orders> for Audit {
    ///     fn init(orders: &Orders, _: &JourneyId, _: &DataBag) -> Result<Self, Error> {
    ///         if orders.audited {
    ///             Ok(Audit)
    ///         } else {
    ///             Err(Error::msg("auditing is off"))
    ///         }
    ///     }
    /// }
    /// ```
    fn init(workflow: &W, journey_id: &JourneyId, data: &DataBag) -> Result<Self, Error>;
}

/// Delivers the events of one journey to the reporters added to it.
///
/// An executor gets a new dispatcher for every journey from its [`DispatcherFactory`], adds the
/// journey's reporters to it, dispatches every event of the journey through it, and drops it
/// when the journey ends. What adding a reporter means, and how events are delivered, is the
/// dispatcher's own decision; [`DefaultDispatcher`] is the usual one.
///
/// Failing in [`add`](Dispatcher::add) refuses the journey before it starts; failing in
/// [`dispatch`](Dispatcher::dispatch) aborts it with `reporter failed`, except while
/// `journey_aborted` is dispatched, when the error is ignored.
///
/// # Examples
///
/// A dispatcher that ignores the workflow's reporters and delivers only to its own:
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::report::{Dispatcher, Reporter};
///
/// struct OnlyMine {
///     reporter: Box<dyn Reporter>,
/// }
///
/// impl Dispatcher for OnlyMine {
///     fn add(&mut self, _reporter: Box<dyn Reporter>) -> Result<(), Error> {
///         Ok(())
///     }
///
///     fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
///         self.reporter.report(event)
///     }
/// }
/// ```
pub trait Dispatcher: Send + 'static {
    /// Adds a reporter for the journey.
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
    /// assert!(dispatcher.add(Box::new(Silent)).is_ok());
    /// ```
    fn add(&mut self, reporter: Box<dyn Reporter>) -> Result<(), Error>;

    /// Delivers one event.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::Dispatcher;
    ///
    /// fn deliver(dispatcher: &mut impl Dispatcher, event: &Event) -> Result<(), Error> {
    ///     dispatcher.dispatch(event)
    /// }
    /// ```
    fn dispatch(&mut self, event: &Event) -> Result<(), Error>;
}

/// Creates a new [`Dispatcher`] for every journey.
///
/// Failing refuses the journey before it starts.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::report::{DefaultDispatcher, DispatcherFactory};
///
/// struct Counting {
///     created: u32,
/// }
///
/// impl DispatcherFactory for Counting {
///     type Dispatcher = DefaultDispatcher;
///
///     fn create(&mut self) -> Result<DefaultDispatcher, Error> {
///         self.created += 1;
///         Ok(DefaultDispatcher::new())
///     }
/// }
///
/// let mut factory = Counting { created: 0 };
/// assert!(factory.create().is_ok());
/// assert_eq!(factory.created, 1);
/// ```
pub trait DispatcherFactory: Send + 'static {
    /// The dispatchers it creates.
    type Dispatcher: Dispatcher;

    /// Creates the dispatcher for one journey.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::report::{DefaultDispatcherFactory, DispatcherFactory};
    ///
    /// assert!(DefaultDispatcherFactory.create().is_ok());
    /// ```
    fn create(&mut self) -> Result<Self::Dispatcher, Error>;
}

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
pub struct DefaultDispatcher<R: sealed::Held = Box<dyn Reporter>> {
    reporters: Vec<R>,
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
    /// # drop(dispatcher);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

impl<R: sealed::Held> fmt::Debug for DefaultDispatcher<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultDispatcher")
            .field("reporters", &self.reporters.len())
            .finish()
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

struct Delivery {
    to_everyone: bool,
    error: Option<Error>,
}

impl Delivery {
    fn of(event: &Event) -> Self {
        Self {
            to_everyone: matches!(event.body, EventBody::JourneyAborted { .. }),
            error: None,
        }
    }

    fn failed(&mut self, result: Result<(), Error>) -> bool {
        if let Err(error) = result {
            self.error.get_or_insert(error);
        }
        self.error.is_some() && !self.to_everyone
    }

    fn finish(self) -> Result<(), Error> {
        self.error.map_or(Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::event::JourneyAbort;
    use crate::event::tests::event;

    type Log = Arc<Mutex<Vec<String>>>;

    struct Recording {
        name: &'static str,
        log: Log,
        fails_on: Option<&'static str>,
    }

    impl Recording {
        fn new(name: &'static str, log: &Log) -> Self {
            Self {
                name,
                log: Arc::clone(log),
                fails_on: None,
            }
        }

        fn failing_on(name: &'static str, kind: &'static str, log: &Log) -> Self {
            Self {
                fails_on: Some(kind),
                ..Self::new(name, log)
            }
        }

        fn record(&mut self, event: &Event) -> Result<(), Error> {
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

    fn started() -> Event {
        event(
            1,
            EventBody::JourneyStarted {
                initial_keys: Vec::new(),
            },
        )
    }

    fn aborted() -> Event {
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

    fn succeeded() -> Event {
        event(2, EventBody::JourneySucceeded { decided_by: None })
    }

    fn entries(log: &Log) -> Vec<String> {
        log.lock().unwrap().clone()
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

    #[cfg(feature = "async")]
    mod asynchronous {
        use futures::executor::block_on;

        use super::*;

        #[derive(derive_more::From)]
        struct AsyncRecording {
            recording: Recording,
        }

        impl AsyncReporter for AsyncRecording {
            async fn report(&mut self, event: &Event) -> Result<(), Error> {
                self.recording.record(event)
            }
        }

        fn dispatcher_of(reporters: Vec<BoxedReporter>) -> DefaultDispatcher<BoxedReporter> {
            let mut dispatcher = block_on(AsyncDispatcherFactory::create(
                &mut DefaultDispatcherFactory,
            ))
            .unwrap();
            for reporter in reporters {
                block_on(dispatcher.add(reporter)).unwrap();
            }
            dispatcher
        }

        #[test]
        fn the_async_default_dispatcher_calls_both_kinds_of_reporter_in_order() {
            let log = Log::default();
            let mut dispatcher = dispatcher_of(vec![
                BoxedReporter::from_async_reporter(AsyncRecording::from(Recording::new(
                    "audit", &log,
                ))),
                BoxedReporter::from_reporter(Recording::new("metrics", &log)),
            ]);

            block_on(dispatcher.dispatch(&started())).unwrap();

            assert_eq!(
                entries(&log),
                ["audit journey_started", "metrics journey_started"]
            );
        }

        #[test]
        fn async_delivery_of_an_event_stops_at_the_reporter_that_failed() {
            let log = Log::default();
            let mut dispatcher =
                dispatcher_of(vec![
                    BoxedReporter::from_reporter(Recording::new("audit", &log)),
                    BoxedReporter::from_async_reporter(AsyncRecording::from(
                        Recording::failing_on("fragile", "journey_started", &log),
                    )),
                    BoxedReporter::from_reporter(Recording::new("metrics", &log)),
                ]);

            let error = block_on(dispatcher.dispatch(&started())).unwrap_err();

            assert_eq!(error.to_string(), "fragile failed");
            assert_eq!(entries(&log), ["audit journey_started"]);
        }

        #[test]
        fn a_failure_while_journey_aborted_is_delivered_asynchronously_does_not_stop_its_delivery()
        {
            let log = Log::default();
            let mut dispatcher =
                dispatcher_of(vec![
                    BoxedReporter::from_async_reporter(AsyncRecording::from(
                        Recording::failing_on("fragile", "journey_aborted", &log),
                    )),
                    BoxedReporter::from_reporter(Recording::new("audit", &log)),
                ]);

            assert!(block_on(dispatcher.dispatch(&aborted())).is_err());
            assert_eq!(entries(&log), ["audit journey_aborted"]);
        }
    }
}
