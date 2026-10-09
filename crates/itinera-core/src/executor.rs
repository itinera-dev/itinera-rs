//! Executors, which run workflow instances, and the refusals they give when a journey cannot
//! start.

use crate::engine::{self, Failures, Inline};
use crate::error::Error;
use crate::instance::WorkflowInstance;
use crate::journey::JourneyResult;
use crate::mode::Synchronous;
use crate::report::{DefaultDispatcherFactory, Dispatcher, DispatcherFactory};

#[cfg(feature = "async")]
mod asynchronous;

#[cfg(feature = "async")]
pub use asynchronous::AsyncLocalExecutor;

/// Runs synchronous workflows in process, one journey at a time, with no persistence: a journey
/// lives only as long as the call to [`run`](LocalExecutor::run).
///
/// It is given a [`DispatcherFactory`], or uses [`DefaultDispatcherFactory`]. Nothing of a
/// journey remains in it once `run` returns, so it can run many journeys one after another.
///
/// # Examples
///
/// ```
/// use itinera::executor::LocalExecutor;
/// use itinera::journey::StatusKind;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .build()?;
/// let mut executor = LocalExecutor::new();
/// let result = executor.run(orders.instance(Orders).create()?)?;
/// assert_eq!(result.status.kind(), StatusKind::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct LocalExecutor<F = DefaultDispatcherFactory> {
    factory: F,
}

impl LocalExecutor {
    /// Makes an executor that uses the [`DefaultDispatcherFactory`].
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::LocalExecutor;
    ///
    /// let executor = LocalExecutor::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

impl<F: DispatcherFactory> LocalExecutor<F> {
    /// Makes an executor that creates the dispatcher of every journey with this factory.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::LocalExecutor;
    /// use itinera::report::DefaultDispatcherFactory;
    ///
    /// let executor = LocalExecutor::with_dispatcher_factory(DefaultDispatcherFactory);
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
    /// use itinera::executor::LocalExecutor;
    /// use itinera::journey::JourneyStatus;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    /// let result = LocalExecutor::new().run(instance)?;
    /// let JourneyStatus::Succeeded { data, .. } = result.status else {
    ///     panic!("the journey did not succeed");
    /// };
    /// assert!(data.get("amount").is_some());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn run<I: WorkflowInstance<Mode = Synchronous>>(
        &mut self,
        mut instance: I,
    ) -> Result<JourneyResult, Refusal> {
        let mut dispatcher = self.factory.create().map_err(Refusal::DispatcherFactory)?;
        let failures = Failures::default();
        instance
            .take_reporters()
            .into_iter()
            .map(|reporter| failures.guard(reporter))
            .try_for_each(|reporter| dispatcher.add(reporter))
            .map_err(Refusal::Dispatcher)?;
        Ok(engine::finish(engine::run(
            instance,
            Inline::from(dispatcher),
            failures,
            std::time::SystemTime::now,
        )))
    }
}

/// Why `run` did not start a journey: custom code called before the journey starts failed. There
/// is no journey and no event.
///
/// It names what refused the journey, and carries its error.
///
/// # Examples
///
/// ```
/// use itinera::executor::Refusal;
///
/// fn blames_the_dispatcher(refusal: &Refusal) -> bool {
///     matches!(refusal, Refusal::DispatcherFactory(_) | Refusal::Dispatcher(_))
/// }
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    /// The dispatcher factory failed to create the journey's dispatcher.
    #[error("the dispatcher factory failed: {0}")]
    DispatcherFactory(Error),
    /// The dispatcher failed while the journey's reporters were added.
    #[error("the dispatcher failed while the reporters were added: {0}")]
    Dispatcher(Error),
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::{Arc, Mutex};

    use rstest::rstest;

    use super::*;
    use crate::event::{Event, EventBody, JourneyAbort};
    use crate::instance::Instance;
    use crate::journey::{Abort, DataBag, JourneyId, JourneyStatus};
    use crate::report::{DefaultDispatcher, Reporter, WorkflowReporter};
    use crate::step::{Outcome, StepDescriptor, StepName};
    use crate::workflow::{WorkflowBuilder, WorkflowDescriptor};

    /// What the journey's reporters received, in order, as "reporter kind" lines.
    pub(crate) type Log = Arc<Mutex<Vec<String>>>;

    /// The workflow of these tests: it shares a log with its reporters and its step, and says
    /// on which event each reporter fails.
    pub(crate) struct Shop {
        pub(crate) log: Log,
        pub(crate) fails_on: Vec<(&'static str, &'static str)>,
    }

    impl Shop {
        pub(crate) fn new() -> Self {
            Self {
                log: Log::default(),
                fails_on: Vec::new(),
            }
        }

        pub(crate) fn failing(reporter: &'static str, kind: &'static str) -> Self {
            Self {
                fails_on: vec![(reporter, kind)],
                ..Self::new()
            }
        }
    }

    const NAMES: [&str; 3] = ["audit", "fragile", "metrics"];

    /// A reporter that logs what it receives, and fails as its workflow says.
    pub(crate) struct Recorder<const N: usize> {
        log: Log,
        fails_on: Vec<&'static str>,
    }

    impl<const N: usize> Recorder<N> {
        pub(crate) fn record(&mut self, event: &Event) -> Result<(), Error> {
            let name = NAMES[N];
            self.log
                .lock()
                .unwrap()
                .push(format!("{name} {}", event.kind()));
            if self.fails_on.contains(&event.kind()) {
                return Err(Error::msg(format!("{name} failed on {}", event.kind())));
            }
            Ok(())
        }
    }

    impl<const N: usize> Reporter for Recorder<N> {
        fn report(&mut self, event: &Event) -> Result<(), Error> {
            self.record(event)
        }
    }

    impl<const N: usize> WorkflowReporter<Shop> for Recorder<N> {
        fn init(shop: &Shop, _: &JourneyId, _: &DataBag) -> Result<Self, Error> {
            Ok(Self::new(shop))
        }
    }

    impl<const N: usize> Recorder<N> {
        pub(crate) fn new(shop: &Shop) -> Self {
            let name = NAMES[N];
            let fails_on = shop
                .fails_on
                .iter()
                .filter_map(|failure| kind_failed_on(failure, name))
                .collect();
            Self {
                log: Arc::clone(&shop.log),
                fails_on,
            }
        }
    }

    /// The kind of event a scripted failure is on, if it is a failure of this reporter.
    fn kind_failed_on(
        &(reporter, kind): &(&'static str, &'static str),
        name: &str,
    ) -> Option<&'static str> {
        (reporter == name).then_some(kind)
    }

    pub(crate) fn entries(log: &Log) -> Vec<String> {
        log.lock().unwrap().clone()
    }

    fn shop_builder() -> WorkflowBuilder<Shop> {
        WorkflowDescriptor::builder("shop")
    }

    pub(crate) fn shop_workflow() -> WorkflowDescriptor<Shop> {
        shop_builder()
            .step(StepDescriptor::new(StepName::new("charge"), || {
                Ok(Outcome::success())
            }))
            .reporter::<Recorder<0>>()
            .reporter::<Recorder<1>>()
            .reporter::<Recorder<2>>()
            .id_generator(|_: &Shop, _| Ok("order-7".to_string()))
            .build()
            .unwrap()
    }

    type Execute = fn(Instance<Shop>) -> Result<JourneyResult, Refusal>;

    fn on<F: DispatcherFactory + Default>(
        instance: Instance<Shop>,
    ) -> Result<JourneyResult, Refusal> {
        LocalExecutor::with_dispatcher_factory(F::default()).run(instance)
    }

    fn run_on(execute: Execute, shop: Shop) -> (JourneyResult, Log) {
        let log = Arc::clone(&shop.log);
        let instance = shop_workflow()
            .instance(shop)
            .data("amount", 42_i64)
            .create()
            .unwrap();
        (execute(instance).unwrap(), log)
    }

    fn run(shop: Shop) -> (JourneyResult, Log) {
        run_on(on::<DefaultDispatcherFactory>, shop)
    }

    #[test]
    fn a_journey_emits_its_events_in_order_to_every_reporter_and_succeeds_with_its_data() {
        let (result, log) = run(Shop::new());

        assert_eq!(result.journey_id.to_string(), "order-7");
        let JourneyStatus::Succeeded { data } = result.status else {
            panic!("the journey did not succeed: {:?}", result.status);
        };
        assert_eq!(data.keys().collect::<Vec<_>>(), ["amount"]);
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

    fn count(runs: &Mutex<u32>) -> Result<Outcome, Error> {
        *runs.lock().unwrap() += 1;
        Ok(Outcome::success())
    }

    #[test]
    fn the_step_runs_once() {
        let runs = Arc::new(Mutex::new(0));
        let counted = Arc::clone(&runs);
        let workflow = shop_builder()
            .step(StepDescriptor::new(StepName::new("charge"), move || {
                count(&counted)
            }))
            .build()
            .unwrap();

        let result = LocalExecutor::new()
            .run(workflow.instance(Shop::new()).create().unwrap())
            .unwrap();

        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
        assert_eq!(*runs.lock().unwrap(), 1);
    }

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

    #[test]
    fn a_reporter_that_fails_on_the_last_decision_still_aborts_the_journey() {
        let (result, log) = run(Shop::failing("metrics", "journey_succeeded"));

        assert!(matches!(
            result.status,
            JourneyStatus::Aborted(Abort::ReporterFailed(_))
        ));
        assert_eq!(
            entries(&log)[9..],
            [
                "audit journey_succeeded",
                "fragile journey_succeeded",
                "metrics journey_succeeded",
                "audit journey_aborted",
                "fragile journey_aborted",
            ]
        );
    }

    #[derive(Default)]
    struct Recording {
        events: Arc<Mutex<Vec<Event>>>,
    }

    impl Reporter for Recording {
        fn report(&mut self, event: &Event) -> Result<(), Error> {
            self.events.lock().unwrap().push(event.clone());
            Ok(())
        }
    }

    impl WorkflowReporter<Shop> for Recording {
        fn init(_: &Shop, _: &JourneyId, _: &DataBag) -> Result<Self, Error> {
            Ok(Self::default())
        }
    }

    #[test]
    fn journey_aborted_names_the_step_during_which_a_reporter_failed() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut dispatcher = DefaultDispatcher::new();
        dispatcher
            .add(Box::new(Recording {
                events: Arc::clone(&events),
            }))
            .unwrap();
        let instance = shop_workflow()
            .instance(Shop::failing("fragile", "step_succeeded"))
            .create()
            .unwrap();

        let mut executor = LocalExecutor::with_dispatcher_factory(Holding {
            dispatcher: Some(dispatcher),
        });
        executor.run(instance).unwrap();

        let events = events.lock().unwrap();
        let Some(EventBody::JourneyAborted {
            abort: JourneyAbort::ReporterFailed { step, error },
        }) = events.last().map(|event| &event.body)
        else {
            panic!("the last event is not journey_aborted: {events:?}");
        };
        assert_eq!(*step, Some(StepName::new("charge")));
        assert_eq!(error, "fragile failed on step_succeeded");
    }

    /// A factory whose dispatcher holds the test's reporter first, then the journey's.
    struct Holding {
        dispatcher: Option<DefaultDispatcher>,
    }

    impl DispatcherFactory for Holding {
        type Dispatcher = DefaultDispatcher;

        fn create(&mut self) -> Result<DefaultDispatcher, Error> {
            self.dispatcher
                .take()
                .ok_or_else(|| Error::msg("used twice"))
        }
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

    /// A dispatcher that fails on one kind of event, or when a reporter is added.
    struct Failing {
        on: Option<&'static str>,
        dispatcher: DefaultDispatcher,
    }

    impl Dispatcher for Failing {
        fn add(&mut self, reporter: Box<dyn Reporter>) -> Result<(), Error> {
            match self.on {
                None => Err(Error::msg("the dispatcher refuses reporters")),
                Some(_) => self.dispatcher.add(reporter),
            }
        }

        fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
            self.dispatcher.dispatch(event)?;
            match self.on {
                Some(kind) if kind == event.kind() => {
                    Err(Error::msg(format!("the dispatcher failed on {kind}")))
                }
                _ => Ok(()),
            }
        }
    }

    struct FailingFactory {
        on: Option<Option<&'static str>>,
    }

    impl DispatcherFactory for FailingFactory {
        type Dispatcher = Failing;

        fn create(&mut self) -> Result<Failing, Error> {
            match self.on {
                None => Err(Error::msg("no dispatcher today")),
                Some(on) => Ok(Failing {
                    on,
                    dispatcher: DefaultDispatcher::new(),
                }),
            }
        }
    }

    fn run_with(factory: FailingFactory) -> (Result<JourneyResult, Refusal>, Log) {
        run_shop_with(Shop::new(), factory)
    }

    fn run_shop_with(shop: Shop, factory: FailingFactory) -> (Result<JourneyResult, Refusal>, Log) {
        let log = Arc::clone(&shop.log);
        let instance = shop_workflow().instance(shop).create().unwrap();
        (
            LocalExecutor::with_dispatcher_factory(factory).run(instance),
            log,
        )
    }

    #[test]
    fn a_dispatcher_that_fails_while_dispatching_aborts_the_journey() {
        let (result, log) = run_with(FailingFactory {
            on: Some(Some("step_succeeded")),
        });

        let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = result.unwrap().status else {
            panic!("the journey was not aborted by the dispatcher");
        };
        assert_eq!(error.to_string(), "the dispatcher failed on step_succeeded");
        assert_eq!(
            entries(&log)[6..],
            [
                "audit step_succeeded",
                "fragile step_succeeded",
                "metrics step_succeeded",
                "audit journey_aborted",
                "fragile journey_aborted",
                "metrics journey_aborted",
            ]
        );
    }

    #[test]
    fn a_dispatcher_that_fails_while_journey_aborted_is_delivered_is_ignored() {
        let (result, log) = run_shop_with(
            Shop::failing("fragile", "journey_started"),
            FailingFactory {
                on: Some(Some("journey_aborted")),
            },
        );

        let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = result.unwrap().status else {
            panic!("the journey was not aborted by a reporter");
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

    #[test]
    fn a_dispatcher_factory_that_fails_refuses_the_journey_before_any_event() {
        let (result, log) = run_with(FailingFactory { on: None });

        let Err(Refusal::DispatcherFactory(error)) = result else {
            panic!("the journey was not refused by the factory");
        };
        assert_eq!(error.to_string(), "no dispatcher today");
        assert!(entries(&log).is_empty());
    }

    #[test]
    fn a_dispatcher_that_fails_while_reporters_are_added_refuses_the_journey_before_any_event() {
        let (result, log) = run_with(FailingFactory { on: Some(None) });

        let Err(refusal) = result else {
            panic!("the journey was not refused");
        };
        assert!(matches!(refusal, Refusal::Dispatcher(_)));
        assert_eq!(
            refusal.to_string(),
            "the dispatcher failed while the reporters were added: the dispatcher refuses reporters"
        );
        assert!(entries(&log).is_empty());
    }

    struct Counting {
        created: usize,
    }

    impl DispatcherFactory for Counting {
        type Dispatcher = DefaultDispatcher;

        fn create(&mut self) -> Result<DefaultDispatcher, Error> {
            self.created += 1;
            Ok(DefaultDispatcher::new())
        }
    }

    #[test]
    fn an_executor_runs_journeys_one_after_another_each_with_a_new_dispatcher() {
        let workflow = shop_builder()
            .step(StepDescriptor::new(StepName::new("charge"), || {
                Ok(Outcome::success())
            }))
            .build()
            .unwrap();
        let mut executor = LocalExecutor::with_dispatcher_factory(Counting { created: 0 });

        let first = executor
            .run(workflow.instance(Shop::new()).create().unwrap())
            .unwrap();
        let second = executor
            .run(workflow.instance(Shop::new()).create().unwrap())
            .unwrap();

        assert_ne!(first.journey_id, second.journey_id);
        assert_eq!(executor.factory.created, 2);
    }

    #[test]
    fn a_journey_without_a_step_succeeds() {
        let shop = Shop::new();
        let log = Arc::clone(&shop.log);
        let workflow = shop_builder().reporter::<Recorder<0>>().build().unwrap();

        let result = LocalExecutor::new()
            .run(workflow.instance(shop).create().unwrap())
            .unwrap();

        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
        assert_eq!(
            entries(&log),
            ["audit journey_started", "audit journey_succeeded"]
        );
    }
}
