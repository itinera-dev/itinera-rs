use super::Refusal;
use crate::engine::{self, Awaited, Failures};
use crate::instance::WorkflowInstance;
use crate::journey::JourneyResult;
use crate::mode::sealed::Sealed;
use crate::report::{AsyncDispatcher, AsyncDispatcherFactory, DefaultDispatcherFactory};

/// Runs workflows of either mode in process, one journey at a time, with no persistence:
/// everything asynchronous is awaited, and synchronous parts run inline.
///
/// It is given an [`AsyncDispatcherFactory`], or uses [`DefaultDispatcherFactory`]. Nothing of a
/// journey remains in it once `run` returns, so it can run many journeys one after another. It
/// needs no particular async runtime: it never spawns, sleeps or sets timers.
///
/// Dropping the future that [`run`](AsyncLocalExecutor::run) returns abandons the journey where
/// it is.
///
/// # Examples
///
/// ```
/// use itinera::executor::AsyncLocalExecutor;
/// use itinera::journey::StatusKind;
/// use itinera::step::{StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// async fn charge() -> Result<StatusKind, Box<dyn std::error::Error>> {
///     let orders = WorkflowDescriptor::builder("orders")
///         .step(StepDescriptor::new(step_name!("charge"), || {}))
///         .build()?;
///     let mut executor = AsyncLocalExecutor::new();
///     let result = executor.run(orders.instance(Orders).create()?).await?;
///     Ok(result.status.kind())
/// }
///
/// assert_eq!(futures::executor::block_on(charge())?, StatusKind::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct AsyncLocalExecutor<F = DefaultDispatcherFactory> {
    factory: F,
}

impl AsyncLocalExecutor {
    /// Makes an executor that uses the [`DefaultDispatcherFactory`].
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::AsyncLocalExecutor;
    ///
    /// let executor = AsyncLocalExecutor::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

impl<F: AsyncDispatcherFactory> AsyncLocalExecutor<F> {
    /// Makes an executor that creates the dispatcher of every journey with this factory.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::AsyncLocalExecutor;
    /// use itinera::report::DefaultDispatcherFactory;
    ///
    /// let executor = AsyncLocalExecutor::with_dispatcher_factory(DefaultDispatcherFactory);
    /// ```
    pub fn with_dispatcher_factory(factory: F) -> Self {
        Self { factory }
    }

    /// Runs one journey of the instance and returns its result.
    ///
    /// Before the journey starts, it creates the journey's dispatcher and adds the instance's
    /// reporters to it. Whatever happens inside the journey, failures and aborts included, is in
    /// the result. A panic is not caught: it reaches the caller, and the journey stops where it
    /// was.
    ///
    /// # Errors
    ///
    /// A [`Refusal`], with no journey and no event, when the dispatcher factory fails, or the
    /// dispatcher fails while the reporters are added.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::AsyncLocalExecutor;
    /// use itinera::journey::{JourneyResult, StatusKind};
    /// use itinera::step::{StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// async fn charge() -> Result<JourneyResult, Box<dyn std::error::Error>> {
    ///     let orders = WorkflowDescriptor::builder("orders")
    ///         .step(StepDescriptor::new(step_name!("charge"), || {}))
    ///         .build()?;
    ///     let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    ///     Ok(AsyncLocalExecutor::new().run(instance).await?)
    /// }
    ///
    /// let result = futures::executor::block_on(charge())?;
    /// assert_eq!(result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub async fn run<I: WorkflowInstance>(
        &mut self,
        mut instance: I,
    ) -> Result<JourneyResult, Refusal> {
        let mut dispatcher = self
            .factory
            .create()
            .await
            .map_err(Refusal::DispatcherFactory)?;
        let failures = Failures::default();
        for reporter in instance.take_reporters() {
            let reporter = failures.guard_boxed(<I::Mode as Sealed>::boxed(reporter));
            dispatcher
                .add(reporter)
                .await
                .map_err(Refusal::Dispatcher)?;
        }
        Ok(engine::run(
            instance,
            Awaited::from(dispatcher),
            failures,
            std::time::SystemTime::now,
        )
        .await)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures::executor::block_on;

    use super::*;
    use crate::error::Error;
    use crate::event::Event;
    use crate::executor::tests::{Recorder, Shop, entries, shop_workflow};
    use crate::journey::{Abort, DataBag, JourneyId, JourneyStatus};
    use crate::mode::Asynchronous;
    use crate::report::{AsyncReporter, AsyncWorkflowReporter, BoxedReporter, DefaultDispatcher};
    use crate::step::{StepDescriptor, StepName};
    use crate::workflow::WorkflowDescriptor;

    /// An asynchronous reporter that logs and fails like the synchronous one it wraps.
    struct AsyncRecorder<const N: usize> {
        recorder: Recorder<N>,
    }

    impl<const N: usize> AsyncReporter for AsyncRecorder<N> {
        async fn report(&mut self, event: &Event) -> Result<(), Error> {
            self.recorder.record(event)
        }
    }

    impl<const N: usize> AsyncWorkflowReporter<Shop> for AsyncRecorder<N> {
        fn init(shop: &Shop, _: &JourneyId, _: &DataBag) -> Result<Self, Error> {
            Ok(Self {
                recorder: Recorder::new(shop),
            })
        }
    }

    fn mixed_workflow() -> WorkflowDescriptor<Shop, Asynchronous> {
        WorkflowDescriptor::builder("shop")
            .step(StepDescriptor::new(StepName::new("charge"), || {}))
            .reporter::<Recorder<0>>()
            .async_reporter::<AsyncRecorder<1>>()
            .reporter::<Recorder<2>>()
            .build()
            .unwrap()
    }

    fn assert_send<T: Send>(value: T) -> T {
        value
    }

    #[test]
    fn an_asynchronous_workflow_emits_its_events_in_order_to_both_kinds_of_reporter() {
        let shop = Shop::new();
        let log = Arc::clone(&shop.log);
        let instance = mixed_workflow().instance(shop).create().unwrap();
        let mut executor = AsyncLocalExecutor::new();

        let result = block_on(assert_send(executor.run(instance))).unwrap();

        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
        assert_eq!(
            entries(&log),
            [
                "audit journey_started",
                "fragile journey_started",
                "metrics journey_started",
                "audit attempt_started",
                "fragile attempt_started",
                "metrics attempt_started",
                "audit step_succeeded",
                "fragile step_succeeded",
                "metrics step_succeeded",
                "audit journey_succeeded",
                "fragile journey_succeeded",
                "metrics journey_succeeded",
            ]
        );
    }

    #[test]
    fn the_asynchronous_executor_runs_synchronous_workflows() {
        let shop = Shop::new();
        let log = Arc::clone(&shop.log);
        let instance = shop_workflow().instance(shop).create().unwrap();

        let result = block_on(AsyncLocalExecutor::new().run(instance)).unwrap();

        assert_eq!(result.journey_id.to_string(), "order-7");
        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
        assert_eq!(entries(&log).len(), 12);
    }

    #[test]
    fn an_asynchronous_reporter_that_fails_aborts_the_journey_and_only_journey_aborted_reaches_the_others()
     {
        let shop = Shop::failing("fragile", "journey_started");
        let log = Arc::clone(&shop.log);
        let instance = mixed_workflow().instance(shop).create().unwrap();

        let result = block_on(AsyncLocalExecutor::new().run(instance)).unwrap();

        let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = result.status else {
            panic!(
                "the journey was not aborted by a reporter: {:?}",
                result.status
            );
        };
        assert_eq!(error.to_string(), "fragile failed on journey_started");
        assert_eq!(
            entries(&log),
            [
                "audit journey_started",
                "fragile journey_started",
                "audit journey_aborted",
                "metrics journey_aborted",
            ]
        );
    }

    struct Refusing;

    impl AsyncDispatcherFactory for Refusing {
        type Dispatcher = DefaultDispatcher<BoxedReporter>;

        async fn create(&mut self) -> Result<Self::Dispatcher, Error> {
            Err(Error::msg("no dispatcher today"))
        }
    }

    struct Closed;

    impl AsyncDispatcher for Closed {
        async fn add(&mut self, _reporter: BoxedReporter) -> Result<(), Error> {
            Err(Error::msg("the dispatcher refuses reporters"))
        }

        async fn dispatch(&mut self, _event: &Event) -> Result<(), Error> {
            Ok(())
        }
    }

    struct ClosedFactory;

    impl AsyncDispatcherFactory for ClosedFactory {
        type Dispatcher = Closed;

        async fn create(&mut self) -> Result<Closed, Error> {
            Ok(Closed)
        }
    }

    #[test]
    fn an_asynchronous_dispatcher_that_fails_while_reporters_are_added_refuses_the_journey() {
        let shop = Shop::new();
        let log = Arc::clone(&shop.log);
        let instance = mixed_workflow().instance(shop).create().unwrap();

        let result =
            block_on(AsyncLocalExecutor::with_dispatcher_factory(ClosedFactory).run(instance));

        let Err(Refusal::Dispatcher(error)) = result else {
            panic!("the journey was not refused by the dispatcher");
        };
        assert_eq!(error.to_string(), "the dispatcher refuses reporters");
        assert!(entries(&log).is_empty());
    }

    #[test]
    fn an_asynchronous_dispatcher_factory_that_fails_refuses_the_journey_before_any_event() {
        let shop = Shop::new();
        let log = Arc::clone(&shop.log);
        let instance = mixed_workflow().instance(shop).create().unwrap();

        let result = block_on(AsyncLocalExecutor::with_dispatcher_factory(Refusing).run(instance));

        assert!(matches!(result, Err(Refusal::DispatcherFactory(_))));
        assert!(entries(&log).is_empty());
    }
}
