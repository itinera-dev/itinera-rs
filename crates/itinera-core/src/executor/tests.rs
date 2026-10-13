use std::sync::{Arc, Mutex};

use super::*;
use crate::error::Error;
use crate::executor::fixtures::{FailingFactory, Shop, run, run_with, shop_builder};
use crate::journey::JourneyStatus;
use crate::policy::fixtures::Quiet;
use crate::policy::{PolicyName, WorkflowPolicyDescriptor};
use crate::report::DefaultDispatcher;
use crate::report::fixtures::entries;
use crate::step::{Outcome, StepDescriptor, StepName};

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
            "audit journey_id",
            "fragile journey_id",
            "metrics journey_id",
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
            "metrics journey_succeeded"
        ]
    );
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
fn a_workflow_policy_that_cannot_be_built_refuses_the_journey_before_its_dispatcher() {
    let broken = WorkflowPolicyDescriptor::fallible("notify", || {
        Err::<Quiet, _>(Error::msg("no mail server"))
    })
    .on_workflow_success();
    let workflow = shop_builder().policy(broken).build().unwrap();
    let mut executor = LocalExecutor::with_dispatcher_factory(Counting { created: 0 });

    let refused = executor.run(workflow.instance(Shop::new()).create().unwrap());

    let Err(refusal) = refused else {
        panic!("the journey was not refused");
    };
    assert!(matches!(
        &refusal,
        Refusal::WorkflowPolicy { policy, .. } if *policy == PolicyName::from("notify")
    ));
    assert_eq!(
        refusal.to_string(),
        "the workflow policy notify could not be built: no mail server"
    );
    assert_eq!(executor.factory.created, 0);
}

#[test]
fn each_journey_gets_new_workflow_policy_instances() {
    let built = Arc::new(Mutex::new(0));
    let counting = Arc::clone(&built);
    let notify = WorkflowPolicyDescriptor::new("notify", move || {
        *counting.lock().unwrap() += 1;
        Quiet
    })
    .on_workflow_success();
    let workflow = shop_builder().policy(notify).build().unwrap();
    let mut executor = LocalExecutor::new();

    for _ in 0..2 {
        executor
            .run(workflow.instance(Shop::new()).create().unwrap())
            .unwrap();
    }

    assert_eq!(*built.lock().unwrap(), 2);
}

#[test]
fn initial_global_metadata_that_sets_the_journey_id_is_refused_before_the_dispatcher_is_made() {
    let workflow = shop_builder().build().unwrap();
    let mut executor = LocalExecutor::with_dispatcher_factory(Counting { created: 0 });
    let instance = workflow
        .instance(Shop::new())
        .reporters_global_metadata("journey_id", "mine".to_string())
        .create()
        .unwrap();

    let refused = executor.run(instance);

    let Err(refusal) = refused else {
        panic!("the journey was not refused");
    };
    assert!(matches!(refusal, Refusal::ReservedMetadataKey));
    assert_eq!(
        refusal.to_string(),
        "the initial global metadata sets journey_id, which only the executor sets"
    );
    assert_eq!(executor.factory.created, 0);
}

#[test]
fn a_dispatcher_that_fails_on_global_metadata_refuses_the_journey_before_any_event() {
    let (result, log) = run_with(FailingFactory {
        on: Some(Some("journey_id")),
    });

    let Err(refusal) = result else {
        panic!("the journey was not refused");
    };
    assert!(matches!(refusal, Refusal::GlobalMetadata(_)));
    assert_eq!(
        refusal.to_string(),
        "the global metadata could not be set: the dispatcher failed on journey_id"
    );
    assert_eq!(
        entries(&log),
        [
            "audit journey_id",
            "fragile journey_id",
            "metrics journey_id"
        ]
    );
}
