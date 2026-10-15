use std::sync::Arc;

use rstest::rstest;

use crate::engine::fixtures::{
    AMOUNT, CHARGE, DECLINE, DRAFT, Ends, Recording, Seen, attempting,
    charge_declined_with_a_contribution, charge_succeeding, crashed, data_of, declined, failing_on,
    instance, kinds, seen, travel, travel_needing, travel_recorded, travel_workflow,
};
use crate::error::Error;
use crate::event::{Event, EventBody, EventKind, HookSource, JourneyAbort, RequestSource, Source};
use crate::instance::WorkflowInstance;
use crate::journey::{Abort, FailureRequester, JourneyStatus, MissingData, Requester};
use crate::policy::{
    HookNeeds, OnFailure, OnStepFailure, OnStepSuccess, OnSuccess, OnWorkflowSuccess, PolicyName,
    Requested, StepFailure, StepHook, StepPolicyDescriptor, StepSuccess, WorkflowHook,
    WorkflowPolicyDescriptor, WorkflowSuccess,
};
use crate::step::{OptionalInput, StepAttempt};
use crate::workflow::fixtures::{Orders, orders};

const COUPON: OptionalInput<String> = OptionalInput::new("coupon");

/// `on step failure`, which requests data from the step and from the workflow, each
/// required and optional, the reason and the optional error, and records what it got.
struct Alarm {
    seen: Seen,
}

impl OnStepFailure<Orders> for Alarm {
    fn needs() -> HookNeeds<StepFailure> {
        HookNeeds::new()
            .from_step(&DECLINE)
            .from_step(&DRAFT)
            .from_workflow(&AMOUNT)
            .from_workflow(&COUPON)
            .reason()
            .optional_error()
    }

    fn on_step_failure(
        &self,
        mut got: Requested<'_, Orders, StepFailure>,
    ) -> Result<OnFailure, Error> {
        let saw = format!(
            "{} {:?} {} {:?} {} {} {} {} {} {}",
            got.from_step(&DECLINE)?,
            got.from_step(&DRAFT)?,
            got.from_workflow(&AMOUNT)?,
            got.from_workflow(&COUPON)?,
            got.reason()?.code(),
            got.optional_error()?.is_some(),
            got.cause(),
            got.step_name(),
            got.attempt(),
            got.journey_id(),
        );
        self.seen.lock().unwrap().push(saw);
        Ok(OnFailure::KeepDefault)
    }
}

#[test]
fn a_step_hook_receives_what_it_requests_from_the_step_and_the_workflow_before_it_runs() {
    let saw = Seen::default();
    let alarm = Arc::clone(&saw);
    let policy = StepPolicyDescriptor::new("alarm", move || Alarm {
        seen: Arc::clone(&alarm),
    })
    .on_step_failure();
    let workflow = orders().step(charge_declined_with_a_contribution().policy(policy));
    let instance = instance(workflow).data("amount", 42_i64).create().unwrap();
    let journey_id = instance.journey_id().clone();

    let (status, events) = travel(instance);

    assert_eq!(
        seen(&saw),
        [format!(
            "insufficient funds None 42 None declined false failure charge 1 {journey_id}"
        )]
    );
    assert!(matches!(status, JourneyStatus::Failed { .. }));
    assert!(status.data().unwrap().get("decline").is_none());
    let hook = HookSource::Step {
        policy: PolicyName::from("alarm"),
        hook: StepHook::OnStepFailure,
        step: StepAttempt::first(CHARGE),
    };
    let absent: Vec<(&str, &RequestSource)> = events.iter().filter_map(absent_key).collect();
    assert_eq!(
        absent,
        [
            ("draft", &RequestSource::Hook(hook.clone())),
            ("coupon", &RequestSource::Hook(hook))
        ]
    );
    assert_eq!(
        kinds(&events)[3..],
        [
            EventKind::StepGivenUp,
            EventKind::OptionalInputAbsent,
            EventKind::OptionalInputAbsent,
            EventKind::HookCalled,
            EventKind::JourneyFailed
        ]
    );
}

fn absent_key(event: &Event) -> Option<(&str, &RequestSource)> {
    match &event.body {
        EventBody::OptionalInputAbsent { key, requester } => Some((key, requester)),
        _ => None,
    }
}

fn alarm_requester() -> Requester {
    Requester::StepHook {
        policy: PolicyName::from("alarm"),
        hook: StepHook::OnStepFailure,
    }
}

fn alarm_failure_requester() -> FailureRequester {
    FailureRequester::StepHook {
        policy: PolicyName::from("alarm"),
        hook: StepHook::OnStepFailure,
    }
}

#[rstest]
#[case::data_from_the_step(
    HookNeeds::new().from_step(&DECLINE),
    declined,
    MissingData::Key { key: "decline".to_string(), requester: alarm_requester() }
)]
#[case::data_from_the_workflow(
    HookNeeds::new().from_workflow(&AMOUNT),
    declined,
    MissingData::Key { key: "amount".to_string(), requester: alarm_requester() }
)]
#[case::the_reason_of_an_abnormal_termination(
    HookNeeds::new().reason(),
    crashed,
    MissingData::Reason { requester: alarm_failure_requester() }
)]
#[case::the_error_of_a_failure(
    HookNeeds::new().error(),
    declined,
    MissingData::Error { requester: alarm_failure_requester() }
)]
fn a_required_request_without_a_value_aborts_the_journey_before_the_hook_runs(
    #[case] needs: HookNeeds<StepFailure>,
    #[case] ends: Ends,
    #[case] expected: MissingData,
) {
    let (status, events) = travel_needing(needs, ends);

    let JourneyStatus::Aborted(Abort::RequiredDataMissing(missing)) = status else {
        panic!("the journey was not aborted for missing data: {status:?}");
    };
    assert_eq!(missing, expected);
    assert!(!kinds(&events).contains(&EventKind::HookCalled));
    assert!(matches!(
        events.last().map(|event| &event.body),
        Some(EventBody::JourneyAborted { abort }) if abort.step() == Some(CHARGE)
    ));
}

#[rstest]
#[case::the_error_before_the_draft(
    HookNeeds::new().error().from_step(&DRAFT),
    &[
        EventKind::JourneyStarted,
        EventKind::AttemptStarted,
        EventKind::StepFailed,
        EventKind::StepGivenUp,
        EventKind::JourneyAborted,
    ]
)]
#[case::the_draft_before_the_error(
    HookNeeds::new().from_step(&DRAFT).error(),
    &[
        EventKind::JourneyStarted,
        EventKind::AttemptStarted,
        EventKind::StepFailed,
        EventKind::StepGivenUp,
        EventKind::OptionalInputAbsent,
        EventKind::JourneyAborted
    ]
)]
fn a_hooks_requests_are_resolved_in_the_order_declared(
    #[case] needs: HookNeeds<StepFailure>,
    #[case] expected: &[EventKind],
) {
    let (status, events) = travel_needing(needs, declined);

    assert!(matches!(
        status,
        JourneyStatus::Aborted(Abort::RequiredDataMissing(MissingData::Error { .. }))
    ));
    assert_eq!(kinds(&events), expected);
}

/// `on workflow success`, which requests the amount from the workflow.
struct Totals;

impl OnWorkflowSuccess<Orders> for Totals {
    fn needs() -> HookNeeds<WorkflowSuccess> {
        HookNeeds::new().from_workflow(&AMOUNT)
    }

    fn on_workflow_success(&self, _: Requested<'_, Orders, WorkflowSuccess>) -> Result<(), Error> {
        Ok(())
    }
}

#[test]
fn a_value_of_another_type_for_a_hook_aborts_the_journey_naming_the_hook() {
    let workflow = orders()
        .step(charge_succeeding())
        .policy(WorkflowPolicyDescriptor::new("totals", || Totals).on_workflow_success());

    let (status, events) = travel(
        instance(workflow)
            .data("amount", "forty".to_string())
            .create()
            .unwrap(),
    );

    let JourneyStatus::Aborted(Abort::WrongType { key, requester }) = status else {
        panic!("the journey was not aborted for a wrong type: {status:?}");
    };
    assert_eq!(key, "amount");
    assert_eq!(
        requester,
        Requester::WorkflowHook {
            policy: PolicyName::from("totals"),
            hook: WorkflowHook::OnWorkflowSuccess,
        }
    );
    assert_eq!(
        kinds(&events),
        [
            EventKind::JourneyStarted,
            EventKind::AttemptStarted,
            EventKind::StepSucceeded,
            EventKind::JourneyAborted
        ]
    );
}

/// `on step success`, which reports and contributes a new attempt number.
struct Renumber;

impl OnStepSuccess<Orders> for Renumber {
    fn needs() -> HookNeeds<StepSuccess> {
        HookNeeds::new().contributor().reporter()
    }

    fn on_step_success(
        &self,
        mut got: Requested<'_, Orders, StepSuccess>,
    ) -> Result<OnSuccess, Error> {
        got.reporter()?.info("renumbering")?;
        got.contributor()?.contribute("attempt", 10_i64);
        Ok(OnSuccess::KeepDefault)
    }
}

#[test]
fn a_hooks_contributions_are_committed_once_it_returns_with_the_hook_as_their_source() {
    let step = attempting(&[])
        .policy(StepPolicyDescriptor::new("renumber", || Renumber).on_step_success());

    let (status, events) = travel_workflow(orders().step(step));

    assert_eq!(data_of(&status), [("attempt".to_string(), 10)]);
    let hook = HookSource::Step {
        policy: PolicyName::from("renumber"),
        hook: StepHook::OnStepSuccess,
        step: StepAttempt::first(CHARGE),
    };
    let sources: Vec<(EventKind, &Source)> = events.iter().filter_map(committed).collect();
    assert_eq!(
        sources,
        [
            (
                EventKind::ContributionCommitted,
                &Source::Step(StepAttempt::first(CHARGE))
            ),
            (
                EventKind::ContributionCommitted,
                &Source::Hook(hook.clone())
            ),
            (EventKind::DataOverwritten, &Source::Hook(hook.clone())),
        ]
    );
    assert_eq!(
        kinds(&events)[4..],
        [
            EventKind::JourneyInfo,
            EventKind::HookCalled,
            EventKind::ContributionCommitted,
            EventKind::DataOverwritten,
            EventKind::JourneySucceeded
        ]
    );
    assert!(matches!(
        events.get(4).map(|event| &event.body),
        Some(EventBody::JourneyInfo { hook: stamped, message, .. })
            if *stamped == hook && message == "renumbering"
    ));
}

fn committed(event: &Event) -> Option<(EventKind, &Source)> {
    match &event.body {
        EventBody::ContributionCommitted { source, .. }
        | EventBody::DataOverwritten { source, .. } => Some((event.kind(), source)),
        _ => None,
    }
}

#[test]
fn a_reporter_failing_on_a_hooks_event_aborts_the_journey_whatever_the_hook_does_next() {
    let step = attempting(&[])
        .policy(StepPolicyDescriptor::new("renumber", || Renumber).on_step_success());
    let recording = Recording::default();
    let events = Arc::clone(&recording.events);
    let instance = instance(orders().step(step)).create().unwrap();

    let status = travel_recorded(instance, recording, Some(failing_on("journey_info")));

    assert!(matches!(
        status,
        JourneyStatus::Aborted(Abort::ReporterFailed(_))
    ));
    let events = events.lock().unwrap().clone();
    assert_eq!(
        kinds(&events)[4..],
        [EventKind::JourneyInfo, EventKind::JourneyAborted]
    );
}

/// `on step success`, which fails.
struct Ledger;

impl OnStepSuccess<Orders> for Ledger {
    fn on_step_success(&self, _: Requested<'_, Orders, StepSuccess>) -> Result<OnSuccess, Error> {
        Err(Error::msg("the ledger is closed"))
    }
}

#[test]
fn a_hook_that_fails_aborts_the_journey_with_its_error() {
    let step = charge_succeeding()
        .policy(StepPolicyDescriptor::new("ledger", || Ledger).on_step_success());

    let (status, events) = travel_workflow(orders().step(step));

    let JourneyStatus::Aborted(Abort::HookFailed(error)) = status else {
        panic!("the journey was not aborted by the hook: {status:?}");
    };
    assert_eq!(error.to_string(), "the ledger is closed");
    assert!(matches!(
        events.last().map(|event| &event.body),
        Some(EventBody::JourneyAborted { abort }) if *abort == JourneyAbort::HookFailed {
            step: Some(CHARGE),
            error: "the ledger is closed".to_string(),
        }
    ));
    assert!(!kinds(&events).contains(&EventKind::HookCalled));
}

#[cfg(feature = "async")]
mod asynchronous {
    use super::*;
    use crate::engine::fixtures::{SHIP, async_orders};
    use crate::mode::Asynchronous;
    use crate::policy::AsyncOnStepSuccess;
    use crate::step::{Outcome, StepDescriptor};

    /// `on step success` of an asynchronous workflow, which awaits its own event and
    /// finishes the journey.
    struct Announce;

    impl AsyncOnStepSuccess<Orders> for Announce {
        fn needs() -> HookNeeds<StepSuccess> {
            HookNeeds::new().reporter()
        }

        async fn on_step_success(
            &self,
            mut got: Requested<'_, Orders, StepSuccess, Asynchronous>,
        ) -> Result<OnSuccess, Error> {
            got.reporter()?.info("charged").await?;
            Ok(OnSuccess::FinishWorkflow)
        }
    }

    #[test]
    fn an_asynchronous_hook_is_awaited_and_decides_as_a_synchronous_one() {
        let announce = StepPolicyDescriptor::new_async("announce", || Announce).on_step_success();
        let workflow = async_orders()
            .step(
                StepDescriptor::new_async(CHARGE, async || Ok(Outcome::success())).policy(announce),
            )
            .step(StepDescriptor::new_async(SHIP, async || {
                Err(Error::msg("never run"))
            }))
            .build()
            .unwrap();

        let (status, events) = travel(workflow.instance(Orders).create().unwrap());

        assert!(matches!(status, JourneyStatus::Succeeded { .. }));
        assert_eq!(
            kinds(&events),
            [
                EventKind::JourneyStarted,
                EventKind::AttemptStarted,
                EventKind::StepSucceeded,
                EventKind::JourneyInfo,
                EventKind::HookCalled,
                EventKind::JourneySucceeded
            ]
        );
    }
}
