use std::num::NonZeroU32;

use rstest::rstest;

use super::*;
use crate::engine::fixtures::{
    CHARGE, SHIP, attempting, audit, crashed, data_of, declined, fail_workflow, failed_by_the_hook,
    kinds, skips, succeed, travel_hooked, travel_workflow,
};
use crate::error::Error;
use crate::event::{Event, EventKind};
use crate::journey::{Failure, FailureCause, JourneyStatus, LastFailure};
use crate::policy::Lifecycle;
use crate::step::{Outcome, RescueOutcome, StepDescriptor};
use crate::workflow::fixtures::{Orders, orders};

#[rstest]
#[case::a_failure(declined, EventKind::StepFailed, FailureCause::Failure)]
#[case::an_abnormal_termination(
    crashed,
    EventKind::StepAbnormalTermination,
    FailureCause::AbnormalTermination
)]
fn without_a_retry_budget_a_failed_attempt_is_the_steps_last_and_fails_the_journey(
    #[case] ends: fn() -> Result<Outcome, Error>,
    #[case] fact: EventKind,
    #[case] cause: FailureCause,
) {
    let workflow = orders()
        .step(StepDescriptor::new(CHARGE, ends))
        .step(StepDescriptor::new(SHIP, succeed));

    let (status, events) = travel_workflow(workflow);

    let JourneyStatus::Failed { failure, .. } = status else {
        panic!("the journey did not fail: {status:?}");
    };
    assert_eq!(failure.cause(), cause);
    assert_eq!(
        kinds(&events),
        [
            EventKind::JourneyStarted,
            EventKind::AttemptStarted,
            fact,
            EventKind::StepGivenUp,
            EventKind::JourneyFailed
        ]
    );
    let Some(EventBody::JourneyFailed { step, failure }) = events.last().map(|e| &e.body) else {
        panic!("the last event is not journey_failed");
    };
    assert_eq!((*step, failure.cause()), (CHARGE, cause));
}

fn attempt_of(event: &Event) -> Option<u32> {
    match &event.body {
        EventBody::AttemptStarted { step } => Some(NonZeroU32::from(step.attempt).get()),
        _ => None,
    }
}

fn retry_cause(event: &Event) -> Option<RetryCause> {
    match &event.body {
        EventBody::StepRetrying { cause, .. } => Some(*cause),
        _ => None,
    }
}

#[test]
fn a_failure_is_attempted_again_while_the_budget_allows() {
    let workflow = orders().step(attempting(&[declined, declined]).retry_budget(2));

    let (status, events) = travel_workflow(workflow);

    assert_eq!(data_of(&status), [("attempt".to_string(), 3)]);
    assert_eq!(
        kinds(&events),
        [
            EventKind::JourneyStarted,
            EventKind::AttemptStarted,
            EventKind::StepFailed,
            EventKind::StepRetrying,
            EventKind::AttemptStarted,
            EventKind::StepFailed,
            EventKind::StepRetrying,
            EventKind::AttemptStarted,
            EventKind::StepSucceeded,
            EventKind::ContributionCommitted,
            EventKind::JourneySucceeded
        ]
    );
    let attempts: Vec<_> = events.iter().filter_map(attempt_of).collect();
    assert_eq!(attempts, [1, 2, 3]);
    let causes: Vec<_> = events.iter().filter_map(retry_cause).collect();
    assert_eq!(causes, [RetryCause::Failure; 2]);
}

#[test]
fn a_step_whose_budget_is_spent_is_given_up_with_its_last_failure() {
    let workflow = orders()
        .step(attempting(&[declined, declined, succeed]).retry_budget(1))
        .step(StepDescriptor::new(SHIP, succeed));

    let (status, events) = travel_workflow(workflow);

    let JourneyStatus::Failed { failure, data } = status else {
        panic!("the journey did not fail: {status:?}");
    };
    assert!(matches!(
        failure,
        Failure::RetriesExhausted(LastFailure::Reason(reason)) if reason.code() == "declined"
    ));
    assert!(data.get("attempt").is_none());
    assert_eq!(
        kinds(&events),
        [
            EventKind::JourneyStarted,
            EventKind::AttemptStarted,
            EventKind::StepFailed,
            EventKind::StepRetrying,
            EventKind::AttemptStarted,
            EventKind::StepFailed,
            EventKind::StepGivenUp,
            EventKind::JourneyFailed
        ]
    );
    assert!(matches!(
        events.get(6).map(|e| &e.body),
        Some(EventBody::StepGivenUp { cause: GiveUpCause::RetriesExhausted, step })
            if NonZeroU32::from(step.attempt).get() == 2
    ));
}

#[rstest]
#[case::by_default(attempting(&[crashed]).retry_budget(1), EventKind::StepRetrying)]
#[case::not_when_the_step_turns_it_off(
    attempting(&[crashed]).retry_budget(1).no_retry_on_abnormal_termination(),
    EventKind::StepGivenUp
)]
fn an_abnormal_termination_is_retried_unless_the_step_turns_it_off(
    #[case] step: StepDescriptor<Orders>,
    #[case] decision: EventKind,
) {
    let (_, events) = travel_workflow(orders().step(step));

    assert_eq!(
        kinds(&events).get(..4),
        Some(
            &[
                EventKind::JourneyStarted,
                EventKind::AttemptStarted,
                EventKind::StepAbnormalTermination,
                decision
            ][..]
        )
    );
}

#[test]
fn an_abnormal_termination_is_retried_with_its_own_cause() {
    let step = attempting(&[crashed]).retry_budget(1);

    let (status, events) = travel_workflow(orders().step(step));

    assert!(matches!(status, JourneyStatus::Succeeded { .. }));
    let causes: Vec<_> = events.iter().filter_map(retry_cause).collect();
    assert_eq!(causes, [RetryCause::AbnormalTermination]);
}

#[rstest]
#[case::on_step_retry(
    attempting(&[declined]).retry_budget(1),
    StepHook::OnStepRetry,
    EventKind::StepFailed
)]
#[case::on_step_abnormal_termination(
    attempting(&[crashed]).retry_budget(1),
    StepHook::OnStepAbnormalTermination,
    EventKind::StepAbnormalTermination
)]
fn fail_workflow_from_a_hook_before_the_decision_gives_the_step_up_without_on_step_failure(
    #[case] step: StepDescriptor<Orders>,
    #[case] hook: StepHook,
    #[case] fact: EventKind,
) {
    let hooked = travel_hooked(step, None, hook, fail_workflow());

    failed_by_the_hook(&hooked, hook);
    assert_eq!(
        kinds(&hooked.events),
        [
            EventKind::JourneyStarted,
            EventKind::AttemptStarted,
            fact,
            EventKind::HookCalled,
            EventKind::StepGivenUp,
            EventKind::JourneyFailed
        ]
    );
    assert!(matches!(
        hooked.events.get(4).map(|e| &e.body),
        Some(EventBody::StepGivenUp {
            cause: GiveUpCause::FailWorkflow { decided_by, .. },
            ..
        }) if decided_by.policy == audit() && decided_by.hook.step_hook() == hook
    ));
    assert_eq!(hooked.calls, [(hook, 3)]);
}

#[test]
fn fail_workflow_from_on_step_failure_gives_the_journey_its_reason_after_the_step_is_given_up() {
    let hooked = travel_hooked(
        attempting(&[declined]),
        None,
        StepHook::OnStepFailure,
        fail_workflow(),
    );

    failed_by_the_hook(&hooked, StepHook::OnStepFailure);
    assert_eq!(
        kinds(&hooked.events),
        [
            EventKind::JourneyStarted,
            EventKind::AttemptStarted,
            EventKind::StepFailed,
            EventKind::StepGivenUp,
            EventKind::HookCalled,
            EventKind::JourneyFailed
        ]
    );
    assert!(matches!(
        hooked.events.get(3).map(|e| &e.body),
        Some(EventBody::StepGivenUp {
            cause: GiveUpCause::Failure,
            ..
        })
    ));
    assert_eq!(hooked.calls, [(StepHook::OnStepFailure, 4)]);
}

#[rstest]
#[case::a_failure_with_budget_left(
    attempting(&[declined]).retry_budget(1),
    &[(StepHook::OnStepRetry, 3), (StepHook::OnStepSuccess, 8)]
)]
#[case::a_failure_with_the_budget_spent(
    attempting(&[declined, declined]).retry_budget(1),
    &[(StepHook::OnStepRetry, 3), (StepHook::OnStepFailure, 8)]
)]
#[case::a_failure_without_a_budget(attempting(&[declined]), &[(StepHook::OnStepFailure, 4)])]
#[case::an_abnormal_termination_the_step_does_not_retry(
    attempting(&[crashed]).retry_budget(1).no_retry_on_abnormal_termination(),
    &[(StepHook::OnStepAbnormalTermination, 3), (StepHook::OnStepFailure, 5)]
)]
#[case::an_abnormal_termination_with_budget_left(
    attempting(&[crashed]).retry_budget(1),
    &[
        (StepHook::OnStepAbnormalTermination, 3),
        (StepHook::OnStepRetry, 4),
        (StepHook::OnStepSuccess, 9)
    ]
)]
#[case::an_abnormal_termination_without_a_budget(
    attempting(&[crashed]),
    &[(StepHook::OnStepAbnormalTermination, 3), (StepHook::OnStepFailure, 5)]
)]
#[case::a_skip(attempting(&[skips]), &[])]
fn step_hooks_are_called_after_each_attempt_in_order_around_the_step_decision(
    #[case] step: StepDescriptor<Orders>,
    #[case] calls: &[(StepHook, usize)],
) {
    let hooked = travel_hooked(
        step,
        None,
        StepHook::OnStepSuccess,
        Lifecycle::FinishWorkflow,
    );

    assert_eq!(hooked.calls, calls);
}

#[test]
fn the_rescued_steps_retry_and_failure_hooks_are_not_called_for_the_attempt_it_rescues() {
    let charge = attempting(&[declined]).rescue(|| Ok(RescueOutcome::success()));

    let hooked = travel_hooked(
        charge,
        None,
        StepHook::OnStepFailure,
        Lifecycle::FinishWorkflow,
    );

    assert!(hooked.status.is_succeeded());
    assert_eq!(hook_names(&hooked.calls), [StepHook::OnStepSuccess]);
}

fn hook_names(calls: &[(StepHook, usize)]) -> Vec<StepHook> {
    calls.iter().map(|(hook, _)| *hook).collect()
}
