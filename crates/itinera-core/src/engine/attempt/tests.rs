use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::engine::fixtures::{
    CHARGE, Charging, Counted, DRAFT, Read, Seen, attempting, broken, charge, crashed, failing_on,
    instance, kinds, scripted, seen, timed_out, travel_before, travel_workflow,
};
use crate::journey::JourneyStatus;
use crate::policy::{
    FailWorkflow, HookNeeds, OnStepAbnormalTermination, Requested, StepAbnormalTermination,
    StepPolicyDescriptor,
};
use crate::step::{Resolved, StepFactory, StepNeeds, StepReporter};
use crate::workflow::fixtures::{Orders, orders};

/// The factory of a step that can never be built.
struct Unbuildable;

impl StepFactory for Unbuildable {
    type Step<'a> = Charging<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
    }

    fn build<'a>(&'a self, _: &mut Resolved<'a>) -> Result<Charging<'a>, Error> {
        Err(Error::msg("no card reader"))
    }
}

#[test]
fn a_step_whose_factory_fails_aborts_the_journey_as_it_could_not_be_built() {
    let workflow = orders().step(StepDescriptor::new(CHARGE, Unbuildable));

    let (status, events) = travel_workflow(workflow);

    assert!(matches!(
        status,
        JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(_))
    ));
    assert_eq!(
        kinds(&events),
        ["journey_started", "attempt_started", "journey_aborted"]
    );
}

#[test]
fn a_step_that_takes_a_handle_it_did_not_declare_cannot_be_built() {
    let workflow = orders().step(StepDescriptor::new(CHARGE, Undeclaring));

    let (status, _) = travel_workflow(workflow);

    let JourneyStatus::Aborted(Abort::StepCouldNotBeBuilt(error)) = status else {
        panic!("the journey was not aborted as its step could not be built: {status:?}");
    };
    assert_eq!(
        error.to_string(),
        "the step does not declare a step reporter, or took it already"
    );
}

/// The factory of a step that takes a reporter it does not declare.
struct Undeclaring;

impl StepFactory for Undeclaring {
    type Step<'a> = StepReporter<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<StepReporter<'a>, Error> {
        got.reporter()
    }
}

/// `on step abnormal termination`, which records the draft the attempt contributed.
struct Drafts {
    seen: Seen,
}

impl OnStepAbnormalTermination<Orders> for Drafts {
    fn needs() -> HookNeeds<StepAbnormalTermination> {
        HookNeeds::new().optional_from_step(&DRAFT)
    }

    fn on_step_abnormal_termination(
        &self,
        mut got: Requested<'_, Orders, StepAbnormalTermination>,
    ) -> Result<Option<FailWorkflow>, Error> {
        let draft = got.optional_from_step(&DRAFT)?;
        self.seen.lock().unwrap().push(format!("{draft:?}"));
        Ok(None)
    }
}

#[test]
fn an_abnormal_termination_carries_no_contributions_to_its_hooks() {
    let saw = Seen::default();
    let drafts = Arc::clone(&saw);
    let policy = StepPolicyDescriptor::new("drafts", move || Drafts {
        seen: Arc::clone(&drafts),
    })
    .on_step_abnormal_termination();
    let step = scripted(|contributor, _| {
        contributor.contribute("draft", "D-1".to_string());
        crashed()
    });

    let (_, events) = travel_workflow(orders().step(step.policy(policy)));

    assert_eq!(seen(&saw), ["None"]);
    assert!(kinds(&events).contains(&"optional_input_absent"));
}

#[test]
fn step_policies_are_built_for_every_attempt() {
    let built = Arc::new(AtomicUsize::new(0));
    let counting = Arc::clone(&built);
    let policy = StepPolicyDescriptor::new("counted", move || {
        counting.fetch_add(1, Ordering::SeqCst);
        Counted
    })
    .on_step_retry();
    let step = attempting(&[timed_out]).retry_budget(1).policy(policy);

    let (status, _) = travel_workflow(orders().step(step));

    assert!(matches!(status, JourneyStatus::Succeeded { .. }));
    assert_eq!(built.load(Ordering::SeqCst), 2);
}

#[test]
fn a_step_policy_that_cannot_be_built_aborts_the_journey_before_the_steps_inputs() {
    let step = charge(&Read::default()).policy(broken());

    let (status, events) = travel_workflow(orders().step(step));

    let JourneyStatus::Aborted(Abort::PolicyCouldNotBeBuilt { policy, error }) = status else {
        panic!("the journey was not aborted by the policy: {status:?}");
    };
    assert_eq!(policy, PolicyName::from("counted"));
    assert_eq!(error.to_string(), "no counter");
    assert_eq!(
        kinds(&events),
        ["journey_started", "attempt_started", "journey_aborted"]
    );
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

#[cfg(feature = "async")]
mod asynchronous {
    use super::*;
    use crate::engine::fixtures::{Charge, async_orders, reads, travel};

    #[test]
    fn an_asynchronous_step_is_built_with_its_inputs_and_awaited() {
        let read = Read::default();
        let charge = Charge {
            read: Arc::clone(&read),
        };
        let workflow = async_orders()
            .step(StepDescriptor::new_async(CHARGE, charge))
            .build()
            .unwrap();
        let journey = workflow.instance(Orders).data("amount", 42_i64);

        let (status, _) = travel(journey.create().unwrap());

        assert!(matches!(status, JourneyStatus::Succeeded { .. }));
        assert_eq!(reads(&read), [(42, None)]);
    }
}
