//! The shop the executors' tests run: a workflow whose reporters log what they receive and fail
//! as it says, and dispatchers that fail.

use std::sync::Arc;

use super::{LocalExecutor, Refusal};
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
use crate::workflow::{WorkflowBuilder, WorkflowDescriptor};

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

/// A reporter that logs what it receives, and fails as its workflow says.
pub(crate) struct Recorder {
    name: &'static str,
    log: Log,
    fails_on: Vec<&'static str>,
}

impl Recorder {
    fn record(&mut self, event: &Event) -> Result<(), Error> {
        let name = self.name;
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

impl Reporter for Recorder {
    fn report(&mut self, event: &Event) -> Result<(), Error> {
        self.record(event)
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
    shop_builder()
        .step(StepDescriptor::new(StepName::new("charge"), || {
            Ok(Outcome::success())
        }))
        .id_generator(|_: &Shop, _| Ok("order-7".to_string()))
        .build()
        .unwrap()
}

/// An instance of the shop's workflow, with its three recorders, in order.
pub(crate) fn recorded(shop: Shop) -> InstanceBuilder<Shop> {
    let audit = Recorder::new(&shop, "audit");
    let fragile = Recorder::new(&shop, "fragile");
    let metrics = Recorder::new(&shop, "metrics");
    shop_workflow()
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
    let log = Arc::clone(&shop.log);
    let instance = recorded(shop).data("amount", 42_i64).create().unwrap();
    (execute(instance).unwrap(), log)
}

pub(crate) fn run(shop: Shop) -> (JourneyResult, Log) {
    run_on(on::<DefaultDispatcherFactory>, shop)
}

/// A dispatcher that fails on one kind of event, or when a reporter is added.
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
        match self.on {
            Some(kind) if kind == event.kind() => {
                Err(Error::msg(format!("the dispatcher failed on {kind}")))
            }
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
    async fn report(&mut self, event: &Event) -> Result<(), Error> {
        self.recorder.record(event)
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
