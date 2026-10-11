//! Asynchronous dispatchers and their factories, and the default dispatcher in that mode.

use std::future::Future;

use super::BoxedReporter;
use crate::error::Error;
use crate::event::Event;
use crate::report::default_dispatcher::Delivery;
use crate::report::{DefaultDispatcher, DefaultDispatcherFactory};

/// A [`Dispatcher`](crate::report::Dispatcher) whose operations are asynchronous, and which
/// holds reporters of either kind. The asynchronous executor uses one.
///
/// What [`Dispatcher`](crate::report::Dispatcher) says about failing applies to it too.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::report::{AsyncDispatcher, BoxedReporter};
///
/// struct OnlyMine {
///     reporter: BoxedReporter,
/// }
///
/// impl AsyncDispatcher for OnlyMine {
///     async fn add(&mut self, _reporter: BoxedReporter) -> Result<(), Error> {
///         Ok(())
///     }
///
///     async fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
///         self.reporter.report(event).await
///     }
/// }
/// ```
pub trait AsyncDispatcher: Send + 'static {
    /// Adds a reporter for the journey.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::report::{AsyncDispatcher, BoxedReporter};
    ///
    /// async fn add_all(
    ///     dispatcher: &mut impl AsyncDispatcher,
    ///     reporters: Vec<BoxedReporter>,
    /// ) -> Result<(), Error> {
    ///     for reporter in reporters {
    ///         dispatcher.add(reporter).await?;
    ///     }
    ///     Ok(())
    /// }
    /// ```
    fn add(&mut self, reporter: BoxedReporter) -> impl Future<Output = Result<(), Error>> + Send;

    /// Delivers one event.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::AsyncDispatcher;
    ///
    /// async fn deliver(dispatcher: &mut impl AsyncDispatcher, event: &Event) -> Result<(), Error> {
    ///     dispatcher.dispatch(event).await
    /// }
    /// ```
    fn dispatch(&mut self, event: &Event) -> impl Future<Output = Result<(), Error>> + Send;
}

/// Creates a new [`AsyncDispatcher`] for every journey run by the asynchronous executor.
///
/// Failing refuses the journey before it starts.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::report::{AsyncDispatcherFactory, BoxedReporter, DefaultDispatcher};
///
/// struct Fresh;
///
/// impl AsyncDispatcherFactory for Fresh {
///     type Dispatcher = DefaultDispatcher<BoxedReporter>;
///
///     async fn create(&mut self) -> Result<Self::Dispatcher, Error> {
///         Ok(DefaultDispatcher::default())
///     }
/// }
/// ```
pub trait AsyncDispatcherFactory: Send + 'static {
    /// The dispatchers it creates.
    type Dispatcher: AsyncDispatcher;

    /// Creates the dispatcher for one journey.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::report::AsyncDispatcherFactory;
    ///
    /// async fn fresh<F: AsyncDispatcherFactory>(factory: &mut F) -> Result<F::Dispatcher, Error> {
    ///     factory.create().await
    /// }
    /// ```
    fn create(&mut self) -> impl Future<Output = Result<Self::Dispatcher, Error>> + Send;
}

impl AsyncDispatcher for DefaultDispatcher<BoxedReporter> {
    async fn add(&mut self, reporter: BoxedReporter) -> Result<(), Error> {
        self.reporters.push(reporter);
        Ok(())
    }

    async fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
        let mut delivery = Delivery::of(event);
        for reporter in &mut self.reporters {
            if delivery.failed(reporter.report(event).await) {
                break;
            }
        }
        delivery.finish()
    }
}

impl AsyncDispatcherFactory for DefaultDispatcherFactory {
    type Dispatcher = DefaultDispatcher<BoxedReporter>;

    async fn create(&mut self) -> Result<Self::Dispatcher, Error> {
        Ok(DefaultDispatcher::default())
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use super::*;
    use crate::report::AsyncReporter;
    use crate::report::fixtures::{Log, Recording, aborted, entries, started};

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
            BoxedReporter::from_async_reporter(AsyncRecording::from(Recording::new("audit", &log))),
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
        let mut dispatcher = dispatcher_of(vec![
            BoxedReporter::from_reporter(Recording::new("audit", &log)),
            BoxedReporter::from_async_reporter(AsyncRecording::from(Recording::failing_on(
                "fragile",
                "journey_started",
                &log,
            ))),
            BoxedReporter::from_reporter(Recording::new("metrics", &log)),
        ]);

        let error = block_on(dispatcher.dispatch(&started())).unwrap_err();

        assert_eq!(error.to_string(), "fragile failed");
        assert_eq!(entries(&log), ["audit journey_started"]);
    }

    #[test]
    fn a_failure_while_journey_aborted_is_delivered_asynchronously_does_not_stop_its_delivery() {
        let log = Log::default();
        let mut dispatcher = dispatcher_of(vec![
            BoxedReporter::from_async_reporter(AsyncRecording::from(Recording::failing_on(
                "fragile",
                "journey_aborted",
                &log,
            ))),
            BoxedReporter::from_reporter(Recording::new("audit", &log)),
        ]);

        assert!(block_on(dispatcher.dispatch(&aborted())).is_err());
        assert_eq!(entries(&log), ["audit journey_aborted"]);
    }
}
