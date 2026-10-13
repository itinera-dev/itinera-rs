//! The shop the executors' tests run: a workflow whose reporters log what they receive and fail
//! as it says, and dispatchers that fail.

use std::sync::Arc;

use super::{LocalExecutor, Refusal};
use crate::engine::fixtures::scripted;
use crate::error::Error;
use crate::event::Event;
use crate::instance::{Instance, InstanceBuilder};
use crate::journey::JourneyResult;
#[cfg(feature = "async")]
use crate::mode::Asynchronous;
#[cfg(feature = "async")]
use crate::report::AsyncReporter;
use crate::report::fixtures::Log;
use crate::report::{
    DefaultDispatcher, DefaultDispatcherFactory, Dispatcher, DispatcherFactory, Reporter,
};
use crate::step::{Outcome, StepDescriptor, StepName};
use crate::value::AnyValue;
use crate::workflow::{WorkflowBuilder, WorkflowDescriptor};

/// The workflow of these tests: it shares a log with its reporters and its step, and says
/// on which event, or key of global metadata, each reporter fails.
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

/// A reporter that logs what it receives, and fails as its workflow says.
pub(crate) struct Recorder {
    name: &'static str,
    log: Log,
    fails_on: Vec<&'static str>,
}

impl Recorder {
    /// Logs what it received, the kind of an event or the key of an entry of global metadata,
    /// and fails if the workflow says so.
    fn record(&mut self, received: &str) -> Result<(), Error> {
        let name = self.name;
        self.log.lock().unwrap().push(format!("{name} {received}"));
        if self.fails_on.contains(&received) {
            return Err(Error::msg(format!("{name} failed on {received}")));
        }
        Ok(())
    }
}

impl Reporter for Recorder {
    fn set_global_metadata(&mut self, key: &str, _value: &AnyValue) -> Result<(), Error> {
        self.record(key)
    }

    fn report(&mut self, event: &Event) -> Result<(), Error> {
        self.record(event.kind())
    }
}

impl Recorder {
    /// The recorder of this name for one journey of this shop.
    pub(crate) fn new(shop: &Shop, name: &'static str) -> Self {
        let fails_on = shop
            .fails_on
            .iter()
            .filter_map(|failure| kind_failed_on(failure, name))
            .collect();
        Self {
            name,
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

pub(crate) fn shop_builder() -> WorkflowBuilder<Shop> {
    WorkflowDescriptor::builder("shop")
}

fn shop_workflow() -> WorkflowDescriptor<Shop> {
    with_order_ids(
        shop_builder().step(StepDescriptor::new(StepName::new("charge"), || {
            Ok(Outcome::success())
        })),
    )
}

/// The shop's workflow, whose step sets `user_id` in the global metadata, then succeeds.
fn tagging_workflow() -> WorkflowDescriptor<Shop> {
    with_order_ids(shop_builder().step(scripted(|_, reporter| {
        reporter.set_global_metadata("user_id", "u-7".to_string())?;
        Ok(Outcome::success())
    })))
}

fn with_order_ids(builder: WorkflowBuilder<Shop>) -> WorkflowDescriptor<Shop> {
    builder
        .id_generator(|_: &Shop, _| Ok("order-7".to_string()))
        .build()
        .unwrap()
}

/// An instance of the shop's workflow, with its three recorders, in order.
pub(crate) fn recorded(shop: Shop) -> InstanceBuilder<Shop> {
    recorded_in(shop_workflow(), shop)
}

/// An instance of the shop's workflow whose step sets `user_id`, with its three recorders.
pub(crate) fn tagged(shop: Shop) -> InstanceBuilder<Shop> {
    recorded_in(tagging_workflow(), shop)
}

fn recorded_in(workflow: WorkflowDescriptor<Shop>, shop: Shop) -> InstanceBuilder<Shop> {
    let audit = Recorder::new(&shop, "audit");
    let fragile = Recorder::new(&shop, "fragile");
    let metrics = Recorder::new(&shop, "metrics");
    workflow
        .instance(shop)
        .reporter(audit)
        .reporter(fragile)
        .reporter(metrics)
}

pub(crate) type Execute = fn(Instance<Shop>) -> Result<JourneyResult, Refusal>;

pub(crate) fn on<F: DispatcherFactory + Default>(
    instance: Instance<Shop>,
) -> Result<JourneyResult, Refusal> {
    LocalExecutor::with_dispatcher_factory(F::default()).run(instance)
}

pub(crate) fn run_on(execute: Execute, shop: Shop) -> (JourneyResult, Log) {
    let (result, log) = attempt_on(execute, shop);
    (result.unwrap(), log)
}

/// Runs a journey of the shop, which the executor may refuse.
pub(crate) fn attempt_on(execute: Execute, shop: Shop) -> (Result<JourneyResult, Refusal>, Log) {
    let log = Arc::clone(&shop.log);
    let instance = recorded(shop).data("amount", 42_i64).create().unwrap();
    (execute(instance), log)
}

/// Runs a journey of the shop whose step sets `user_id`.
pub(crate) fn run_tagged_on(execute: Execute, shop: Shop) -> (JourneyResult, Log) {
    let log = Arc::clone(&shop.log);
    let instance = tagged(shop).create().unwrap();
    (execute(instance).unwrap(), log)
}

pub(crate) fn run(shop: Shop) -> (JourneyResult, Log) {
    run_on(on::<DefaultDispatcherFactory>, shop)
}

/// A dispatcher that fails on one kind of event or key of global metadata, or when a reporter is
/// added.
pub(crate) struct Failing {
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
        self.fails_on(event.kind())
    }

    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        self.dispatcher.set_global_metadata(key, value)?;
        self.fails_on(key)
    }
}

impl Failing {
    fn fails_on(&self, received: &str) -> Result<(), Error> {
        match self.on {
            Some(on) if on == received => Err(Error::msg(format!("the dispatcher failed on {on}"))),
            _ => Ok(()),
        }
    }
}

pub(crate) struct FailingFactory {
    pub(crate) on: Option<Option<&'static str>>,
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

pub(crate) fn run_with(factory: FailingFactory) -> (Result<JourneyResult, Refusal>, Log) {
    run_shop_with(Shop::new(), factory)
}

pub(crate) fn run_shop_with(
    shop: Shop,
    factory: FailingFactory,
) -> (Result<JourneyResult, Refusal>, Log) {
    run_instance_with(shop, recorded, factory)
}

/// Runs a journey of the shop whose step sets `user_id`, given the dispatchers of the factory.
pub(crate) fn run_tagged_with(factory: FailingFactory) -> (Result<JourneyResult, Refusal>, Log) {
    run_instance_with(Shop::new(), tagged, factory)
}

fn run_instance_with(
    shop: Shop,
    recorded: fn(Shop) -> InstanceBuilder<Shop>,
    factory: FailingFactory,
) -> (Result<JourneyResult, Refusal>, Log) {
    let log = Arc::clone(&shop.log);
    let instance = recorded(shop).create().unwrap();
    (
        LocalExecutor::with_dispatcher_factory(factory).run(instance),
        log,
    )
}

/// An asynchronous reporter that logs and fails like the synchronous one it wraps.
#[cfg(feature = "async")]
struct AsyncRecorder {
    recorder: Recorder,
}

#[cfg(feature = "async")]
impl AsyncReporter for AsyncRecorder {
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        self.recorder.set_global_metadata(key, value)
    }

    async fn report(&mut self, event: &Event) -> Result<(), Error> {
        self.recorder.report(event)
    }
}

#[cfg(feature = "async")]
fn mixed_workflow() -> WorkflowDescriptor<Shop, Asynchronous> {
    WorkflowDescriptor::async_builder("shop")
        .step(StepDescriptor::new_async(
            StepName::new("charge"),
            async || Ok(Outcome::success()),
        ))
        .build()
        .unwrap()
}

/// An instance of the asynchronous shop, whose second recorder is asynchronous.
#[cfg(feature = "async")]
pub(crate) fn mixed_instance(shop: Shop) -> (Instance<Shop, Asynchronous>, Log) {
    let log = Arc::clone(&shop.log);
    let audit = Recorder::new(&shop, "audit");
    let fragile = AsyncRecorder {
        recorder: Recorder::new(&shop, "fragile"),
    };
    let metrics = Recorder::new(&shop, "metrics");
    let instance = mixed_workflow()
        .instance(shop)
        .reporter(audit)
        .async_reporter(fragile)
        .reporter(metrics)
        .create()
        .unwrap();
    (instance, log)
}
