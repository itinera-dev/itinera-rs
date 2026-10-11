use std::sync::Arc;

use futures::executor::block_on;

use super::*;
use crate::error::Error;
use crate::event::Event;
use crate::executor::fixtures::{Recorder, Shop, entries, mixed_instance};
use crate::journey::JourneyStatus;
use crate::policy::tests::Quiet;
use crate::policy::{PolicyName, WorkflowPolicyDescriptor};
use crate::report::{BoxedReporter, DefaultDispatcher};
use crate::step::{Outcome, StepDescriptor, StepName};
use crate::workflow::WorkflowDescriptor;

fn assert_send<T: Send>(value: T) -> T {
    value
}

#[test]
fn an_asynchronous_workflow_emits_its_events_in_order_to_both_kinds_of_reporter() {
    let (instance, log) = mixed_instance(Shop::new());
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
fn an_asynchronous_workflow_may_list_only_synchronous_reporters() {
    let shop = Shop::new();
    let log = Arc::clone(&shop.log);
    let workflow = WorkflowDescriptor::async_builder("shop")
        .step(StepDescriptor::new_async(
            StepName::new("charge"),
            async || Ok(Outcome::success()),
        ))
        .reporter::<Recorder<0>>()
        .reporter::<Recorder<1>>()
        .id_generator(|_: &Shop, _| Ok("order-7".to_string()))
        .build()
        .unwrap();
    let instance = workflow.instance(shop).create().unwrap();

    let result = block_on(AsyncLocalExecutor::new().run(instance)).unwrap();

    assert_eq!(result.journey_id.to_string(), "order-7");
    assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    assert_eq!(entries(&log).len(), 8);
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
    let (instance, log) = mixed_instance(Shop::new());

    let result = block_on(AsyncLocalExecutor::with_dispatcher_factory(ClosedFactory).run(instance));

    let Err(Refusal::Dispatcher(error)) = result else {
        panic!("the journey was not refused by the dispatcher");
    };
    assert_eq!(error.to_string(), "the dispatcher refuses reporters");
    assert!(entries(&log).is_empty());
}

#[test]
fn an_asynchronous_workflow_policy_that_cannot_be_built_refuses_the_journey_before_any_event() {
    let broken = WorkflowPolicyDescriptor::fallible_async("notify", || {
        Err::<Quiet, _>(Error::msg("no mail server"))
    })
    .on_workflow_success();
    let workflow = WorkflowDescriptor::async_builder("shop")
        .policy(broken)
        .reporter::<Recorder<0>>()
        .build()
        .unwrap();
    let shop = Shop::new();
    let log = Arc::clone(&shop.log);

    let result = block_on(AsyncLocalExecutor::new().run(workflow.instance(shop).create().unwrap()));

    let Err(Refusal::WorkflowPolicy { policy, error }) = result else {
        panic!("the journey was not refused by the policy");
    };
    assert_eq!(policy, PolicyName::from("notify"));
    assert_eq!(error.to_string(), "no mail server");
    assert!(entries(&log).is_empty());
}

#[test]
fn an_asynchronous_dispatcher_factory_that_fails_refuses_the_journey_before_any_event() {
    let (instance, log) = mixed_instance(Shop::new());

    let result = block_on(AsyncLocalExecutor::with_dispatcher_factory(Refusing).run(instance));

    assert!(matches!(result, Err(Refusal::DispatcherFactory(_))));
    assert!(entries(&log).is_empty());
}
