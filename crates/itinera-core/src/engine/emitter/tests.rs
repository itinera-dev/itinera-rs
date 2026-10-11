use std::sync::{Arc, Mutex};

use super::*;
use crate::engine::fixtures::{
    CHARGE, Orders, failing_on, instance, kinds, noon, orders, scripted, succeed, travel_before,
    travel_with_amount, travel_workflow,
};
use crate::event::JourneyAbort;
use crate::executor::LocalExecutor;
use crate::executor::fixtures::{
    FailingFactory, Shop, run, run_shop_with, run_with, shop_workflow,
};
use crate::journey::DataBag;
use crate::report::fixtures::entries;
use crate::report::{DefaultDispatcher, DispatcherFactory, Reporter, WorkflowReporter};
use crate::step::{
    Outcome, Resolved, StepAttempt, StepDescriptor, StepFactory, StepNeeds, StepReporter,
};
use crate::value::AnyValue;

fn sequence(event: &Event) -> NonZeroU64 {
    event.sequence
}

#[test]
fn events_are_numbered_from_one_and_carry_the_journey_the_workflow_and_the_time() {
    let workflow = orders()
        .step(StepDescriptor::new(CHARGE, succeed))
        .id_generator(|_: &Orders, _| Ok("order-7".to_string()));

    let (_, events) = travel_with_amount(workflow);

    let sequences: Vec<u64> = events.iter().map(sequence).map(NonZeroU64::get).collect();
    assert_eq!(sequences, [1, 2, 3, 4]);
    for event in &events {
        assert_eq!(event.journey_id.to_string(), "order-7");
        assert_eq!(event.workflow, WorkflowName::from("orders"));
        assert_eq!(event.timestamp, Timestamp::from(noon()));
    }
    let Some(EventBody::JourneyStarted { initial_keys }) = events.first().map(|e| &e.body) else {
        panic!("the first event is not journey_started");
    };
    assert_eq!(initial_keys, &["amount"]);
}

#[test]
fn a_step_emits_its_own_events_stamped_with_its_attempt_before_its_outcome() {
    let workflow = orders().step(scripted(|_, reporter| {
        reporter.info("charging")?;
        reporter.warning_with("slow", 300_i64)?;
        reporter.error("no receipt")?;
        Ok(Outcome::success())
    }));

    let (_, events) = travel_workflow(workflow);

    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "step_info",
            "step_warning",
            "step_error",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
    let Some(EventBody::StepWarning {
        step,
        message,
        data,
    }) = events.get(3).map(|e| &e.body)
    else {
        panic!("the fourth event is not step_warning");
    };
    assert_eq!(step, &StepAttempt::first(CHARGE));
    assert_eq!(message, "slow");
    assert_eq!(
        data.as_ref().and_then(AnyValue::downcast_ref),
        Some(&300_i64)
    );
}

#[test]
fn a_reporter_failing_on_a_step_event_aborts_the_journey_whatever_the_step_does_next() {
    let emitted = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&emitted);
    let workflow = orders().step(scripted(move |contributor, reporter| {
        let first = reporter.info("charging");
        let second = reporter.warning("still charging");
        seen.lock().unwrap().extend([first.is_ok(), second.is_ok()]);
        contributor.contribute("receipt", 1_i64);
        Ok(Outcome::success())
    }));
    let (status, events) = travel_before(
        instance(workflow).create().unwrap(),
        Some(failing_on("step_info")),
    );

    let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = status else {
        panic!("the journey was not aborted as a reporter failed: {status:?}");
    };
    assert_eq!(error.to_string(), "failed on step_info");
    assert_eq!(*emitted.lock().unwrap(), [false, false]);
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "step_info",
            "journey_aborted"
        ]
    );
    let Some(EventBody::JourneyAborted { abort }) = events.last().map(|e| &e.body) else {
        panic!("the last event is not journey_aborted");
    };
    assert_eq!(abort.step(), Some(CHARGE));
}

#[test]
fn a_reporter_failing_on_an_event_emitted_while_the_step_is_built_aborts_the_journey() {
    let workflow = orders().step(StepDescriptor::new(CHARGE, Preparing));

    let (status, events) = travel_before(
        instance(workflow).create().unwrap(),
        Some(failing_on("step_info")),
    );

    assert!(matches!(
        status,
        JourneyStatus::Aborted(Abort::ReporterFailed(_))
    ));
    let Some(EventBody::JourneyAborted { abort }) = events.last().map(|e| &e.body) else {
        panic!("the last event is not journey_aborted");
    };
    assert_eq!(abort.step(), Some(CHARGE));
}

/// The factory of a step that emits while it is built, and fails to build when interrupted.
struct Preparing;

impl StepFactory for Preparing {
    type Step<'a> = StepReporter<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().reporter()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<StepReporter<'a>, Error> {
        let mut reporter = got.reporter()?;
        reporter.info("preparing")?;
        Ok(reporter)
    }
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

#[cfg(feature = "async")]
mod asynchronous {
    use super::*;
    use crate::engine::fixtures::{async_orders, travel};
    use crate::mode::Asynchronous;
    use crate::step::{AsyncStep, AsyncStepFactory, AsyncStepReporter};

    struct Announcing;

    struct Announcer<'a> {
        reporter: AsyncStepReporter<'a>,
    }

    impl AsyncStepFactory for Announcing {
        type Step<'a> = Announcer<'a>;

        fn needs(&self) -> StepNeeds {
            StepNeeds::new().reporter()
        }

        fn build<'a>(
            &'a self,
            got: &mut Resolved<'a, Asynchronous>,
        ) -> Result<Announcer<'a>, Error> {
            Ok(Announcer {
                reporter: got.reporter()?,
            })
        }
    }

    impl AsyncStep for Announcer<'_> {
        async fn run(mut self) -> Result<Outcome, Error> {
            self.reporter.info_with("charging", 42_i64).await?;
            Ok(Outcome::success())
        }
    }

    #[test]
    fn an_asynchronous_step_awaits_its_own_events() {
        let workflow = async_orders()
            .step(StepDescriptor::new_async(CHARGE, Announcing))
            .build()
            .unwrap();

        let (_, events) = travel(workflow.instance(Orders).create().unwrap());

        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_info",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
    }

    #[test]
    fn a_reporter_failing_on_an_asynchronous_steps_event_aborts_the_journey() {
        let workflow = async_orders()
            .step(StepDescriptor::new_async(CHARGE, Announcing))
            .build()
            .unwrap();

        let (status, events) = travel_before(
            workflow.instance(Orders).create().unwrap(),
            Some(failing_on("step_info")),
        );

        assert!(matches!(
            status,
            JourneyStatus::Aborted(Abort::ReporterFailed(_))
        ));
        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_info",
                "journey_aborted"
            ]
        );
    }
}
