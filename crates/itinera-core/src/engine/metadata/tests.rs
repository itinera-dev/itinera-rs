use std::sync::{Arc, Mutex};

use super::*;
use crate::engine::fixtures::{
    CHARGE, Recording, declined, failing_on, instance, kinds, scripted, succeed,
    travel_with_entries,
};
use crate::event::{Event, HookSource};
use crate::executor::fixtures::{FailingFactory, run_tagged_with};
use crate::journey::JourneyStatus;
use crate::policy::{
    HookNeeds, OnStepSuccess, OnSuccess, PolicyName, Requested, StepHook, StepPolicyDescriptor,
    StepSuccess,
};
use crate::report::fixtures::entries;
use crate::step::{Outcome, StepAttempt, StepDescriptor};
use crate::workflow::fixtures::{Orders, orders};

/// The key and source of `global_metadata_set`, if the event is one.
fn set_by(event: &Event) -> Option<(&str, &Source)> {
    match &event.body {
        EventBody::GlobalMetadataSet { key, source } => Some((key.as_str(), source)),
        _ => None,
    }
}

type Travelled = (JourneyStatus, Vec<Event>, Vec<(String, usize)>);

/// Runs a journey of the orders workflow with this step alone.
fn travel_step(step: StepDescriptor<Orders>, after: Option<Recording>) -> Travelled {
    travel_with_entries(instance(orders().step(step)).create().unwrap(), after)
}

fn keys(entries: &[(String, usize)]) -> Vec<&str> {
    entries.iter().map(|(key, _)| key.as_str()).collect()
}

#[test]
fn the_journey_id_then_the_initial_entries_reach_the_reporters_before_any_event() {
    let journey = instance(orders().step(StepDescriptor::new(CHARGE, succeed)))
        .global_metadata("tenant", "acme".to_string())
        .global_metadata("region", "eu-west".to_string())
        .create()
        .unwrap();

    let (_, events, entries) = travel_with_entries(journey, None);

    assert_eq!(
        entries,
        [
            ("journey_id".to_owned(), 0),
            ("tenant".to_owned(), 0),
            ("region".to_owned(), 0),
        ]
    );
    assert_eq!(events.iter().filter_map(set_by).count(), 0);
}

#[test]
fn a_steps_entry_reaches_the_reporters_then_global_metadata_set_names_its_key_and_step() {
    let step = scripted(|_, reporter| {
        reporter.set_global_metadata("user_id", "u-7".to_string())?;
        Ok(Outcome::success())
    });

    let (_, events, entries) = travel_step(step, None);

    assert_eq!(
        entries,
        [("journey_id".to_owned(), 0), ("user_id".to_owned(), 2)]
    );
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "global_metadata_set",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
    let set: Vec<(&str, &Source)> = events.iter().filter_map(set_by).collect();
    assert_eq!(
        set,
        [("user_id", &Source::Step(StepAttempt::first(CHARGE)))]
    );
}

#[test]
fn an_entry_stands_though_the_attempt_that_set_it_fails() {
    let step = scripted(|_, reporter| {
        reporter.set_global_metadata("user_id", "u-7".to_string())?;
        declined()
    });

    let (status, _, entries) = travel_step(step, None);

    assert!(matches!(status, JourneyStatus::Failed { .. }));
    assert_eq!(keys(&entries), ["journey_id", "user_id"]);
}

#[test]
fn a_step_that_sets_the_journey_id_aborts_the_journey_and_no_reporter_receives_it() {
    let interrupted = Arc::new(Mutex::new(None));
    let seen = Arc::clone(&interrupted);
    let step = scripted(move |_, reporter| {
        let set = reporter.set_global_metadata(GlobalMetadata::JOURNEY_ID, "mine".to_string());
        *seen.lock().unwrap() = Some(set.is_err());
        Ok(Outcome::success())
    });

    let (status, events, entries) = travel_step(step, None);

    let JourneyStatus::Aborted(Abort::ReservedMetadataKey(setter)) = status else {
        panic!("the journey was not aborted for a reserved key: {status:?}");
    };
    assert_eq!(setter, MetadataSetter::Step);
    assert_eq!(*interrupted.lock().unwrap(), Some(true));
    assert_eq!(keys(&entries), ["journey_id"]);
    assert_eq!(
        kinds(&events),
        ["journey_started", "attempt_started", "journey_aborted"]
    );
    let Some(EventBody::JourneyAborted { abort }) = events.last().map(|event| &event.body) else {
        panic!("the last event is not journey_aborted");
    };
    assert_eq!(
        abort,
        &JourneyAbort::ReservedMetadataKey {
            source: Source::Step(StepAttempt::first(CHARGE))
        }
    );
    assert_eq!(abort.step(), Some(CHARGE));
}

#[test]
fn a_reporter_that_fails_on_an_entry_aborts_the_journey_and_interrupts_the_step() {
    let interrupted = Arc::new(Mutex::new(None));
    let seen = Arc::clone(&interrupted);
    let step = scripted(move |_, reporter| {
        let set = reporter.set_global_metadata("user_id", "u-7".to_string());
        *seen.lock().unwrap() = Some(set.is_err());
        Ok(Outcome::success())
    });

    let (status, events, entries) = travel_step(step, Some(failing_on("user_id")));

    let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = status else {
        panic!("the journey was not aborted by a reporter: {status:?}");
    };
    assert_eq!(error.to_string(), "failed on user_id");
    assert_eq!(*interrupted.lock().unwrap(), Some(true));
    assert_eq!(keys(&entries), ["journey_id", "user_id"]);
    assert_eq!(
        kinds(&events),
        ["journey_started", "attempt_started", "journey_aborted"]
    );
}

/// `on step success`, which sets the tenant.
struct Tenant;

impl OnStepSuccess<Orders> for Tenant {
    fn needs() -> HookNeeds<StepSuccess> {
        HookNeeds::new().reporter()
    }

    fn on_step_success(
        &self,
        mut got: Requested<'_, Orders, StepSuccess>,
    ) -> Result<Option<OnSuccess>, Error> {
        got.reporter()?
            .set_global_metadata("tenant", "acme".to_string())?;
        Ok(None)
    }
}

#[test]
fn a_hooks_entry_names_its_policy_and_hook_as_who_set_it() {
    let step = StepDescriptor::new(CHARGE, succeed)
        .policy(StepPolicyDescriptor::new("tenant", || Tenant).on_step_success());

    let (_, events, entries) = travel_step(step, None);

    assert_eq!(keys(&entries), ["journey_id", "tenant"]);
    let hook = Source::Hook(HookSource::Step {
        policy: PolicyName::from("tenant"),
        hook: StepHook::OnStepSuccess,
        step: StepAttempt::first(CHARGE),
    });
    let set: Vec<(&str, &Source)> = events.iter().filter_map(set_by).collect();
    assert_eq!(set, [("tenant", &hook)]);
}

#[cfg(feature = "async")]
mod asynchronous {
    use super::*;
    use crate::engine::fixtures::async_orders;
    use crate::mode::Asynchronous;
    use crate::step::{AsyncStep, AsyncStepFactory, AsyncStepReporter, Resolved, StepNeeds};

    struct Identifying;

    struct Identify<'a> {
        reporter: AsyncStepReporter<'a>,
    }

    impl AsyncStepFactory for Identifying {
        type Step<'a> = Identify<'a>;

        fn needs(&self) -> StepNeeds {
            StepNeeds::new().reporter()
        }

        fn build<'a>(
            &'a self,
            got: &mut Resolved<'a, Asynchronous>,
        ) -> Result<Identify<'a>, Error> {
            Ok(Identify {
                reporter: got.reporter()?,
            })
        }
    }

    impl AsyncStep for Identify<'_> {
        async fn run(mut self) -> Result<Outcome, Error> {
            self.reporter
                .set_global_metadata("user_id", "u-7".to_string())
                .await?;
            Ok(Outcome::success())
        }
    }

    #[test]
    fn an_asynchronous_step_awaits_its_entry() {
        let workflow = async_orders()
            .step(StepDescriptor::new_async(CHARGE, Identifying))
            .build()
            .unwrap();

        let (_, events, entries) =
            travel_with_entries(workflow.instance(Orders).create().unwrap(), None);

        assert_eq!(
            entries,
            [("journey_id".to_owned(), 0), ("user_id".to_owned(), 2)]
        );
        assert_eq!(
            kinds(&events)[2..],
            ["global_metadata_set", "step_succeeded", "journey_succeeded"]
        );
    }
}

#[test]
fn a_dispatcher_that_fails_on_an_entry_a_step_sets_aborts_the_journey() {
    let (result, log) = run_tagged_with(FailingFactory {
        on: Some(Some("user_id")),
    });

    let status = result.unwrap().status;
    let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = status else {
        panic!("the journey was not aborted by the dispatcher: {status:?}");
    };
    assert_eq!(error.to_string(), "the dispatcher failed on user_id");
    assert_eq!(
        entries(&log)[9..],
        [
            "audit user_id",
            "fragile user_id",
            "metrics user_id",
            "audit journey_aborted",
            "fragile journey_aborted",
            "metrics journey_aborted"
        ]
    );
}
