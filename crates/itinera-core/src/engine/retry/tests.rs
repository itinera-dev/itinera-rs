use rstest::rstest;

use super::*;
use crate::engine::fixtures::{
    CHARGE, SHIP, attempting, audit, crashed, data_of, declined, fail_workflow, failed_by_the_hook,
    kinds, skips, succeed, timed_out, travel_hooked, travel_workflow,
};
use crate::error::Error;
use crate::event::Event;
use crate::journey::{Failure, FailureCause, JourneyStatus, LastFailure};
use crate::policy::Lifecycle;
use crate::step::{Outcome, StepDescriptor};
use crate::workflow::fixtures::{Orders, orders};

#[rstest]
#[case::a_failure(declined, "step_failed", FailureCause::Failure)]
#[case::a_retriable_failure(timed_out, "step_failed", FailureCause::RetriesExhausted)]
#[case::an_abnormal_termination(
    crashed,
    "step_abnormal_termination",
    FailureCause::AbnormalTermination
)]
fn without_a_retry_budget_a_failed_attempt_is_the_steps_last_and_fails_the_journey(
    #[case] ends: fn() -> Result<Outcome, Error>,
    #[case] fact: &str,
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
            "journey_started",
            "attempt_started",
            fact,
            "step_given_up",
            "journey_failed"
        ]
    );
    let Some(EventBody::JourneyFailed { step, failure }) = events.last().map(|e| &e.body) else {
        panic!("the last event is not journey_failed");
    };
    assert_eq!((*step, failure.cause()), (CHARGE, cause));
}

fn attempt_of(event: &Event) -> Option<u32> {
    match &event.body {
        EventBody::AttemptStarted { step } => Some(step.attempt.get()),
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
fn a_retriable_failure_is_attempted_again_while_the_budget_allows() {
    let workflow = orders().step(attempting(&[timed_out, timed_out]).retry_budget(2));

    let (status, events) = travel_workflow(workflow);

    assert_eq!(data_of(&status), [("attempt".to_string(), 3)]);
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "step_failed",
            "step_retrying",
            "attempt_started",
            "step_failed",
            "step_retrying",
            "attempt_started",
            "step_succeeded",
            "contribution_committed",
            "journey_succeeded"
        ]
    );
    let attempts: Vec<_> = events.iter().filter_map(attempt_of).collect();
    assert_eq!(attempts, [1, 2, 3]);
    let causes: Vec<_> = events.iter().filter_map(retry_cause).collect();
    assert_eq!(causes, [RetryCause::RetriableFailure; 2]);
}

#[test]
fn a_step_whose_budget_is_spent_is_given_up_with_its_last_failure() {
    let workflow = orders()
        .step(attempting(&[timed_out, timed_out, succeed]).retry_budget(1))
        .step(StepDescriptor::new(SHIP, succeed));

    let (status, events) = travel_workflow(workflow);

    let JourneyStatus::Failed { failure, data } = status else {
        panic!("the journey did not fail: {status:?}");
    };
    assert!(matches!(
        failure,
        Failure::RetriesExhausted(LastFailure::Reason(reason)) if reason.code() == "timeout"
    ));
    assert!(data.get("attempt").is_none());
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "step_failed",
            "step_retrying",
            "attempt_started",
            "step_failed",
            "step_given_up",
            "journey_failed"
        ]
    );
    assert!(matches!(
        events.get(6).map(|e| &e.body),
        Some(EventBody::StepGivenUp { cause: GiveUpCause::RetriesExhausted, step })
            if step.attempt.get() == 2
    ));
}

#[rstest]
#[case::when_the_step_allows_it(
    attempting(&[crashed]).retry_budget(1).abnormal_termination_retriable(),
    "step_retrying"
)]
#[case::not_when_the_step_does_not(attempting(&[crashed]).retry_budget(1), "step_given_up")]
fn an_abnormal_termination_is_retried_only_when_the_step_allows_it(
    #[case] step: StepDescriptor<Orders>,
    #[case] decision: &str,
) {
    let (_, events) = travel_workflow(orders().step(step));

    assert_eq!(
        kinds(&events).get(..4),
        Some(
            &[
                "journey_started",
                "attempt_started",
                "step_abnormal_termination",
                decision
            ][..]
        )
    );
}

#[test]
fn an_abnormal_termination_is_retried_with_its_own_cause() {
    let step = attempting(&[crashed])
        .retry_budget(1)
        .abnormal_termination_retriable();

    let (status, events) = travel_workflow(orders().step(step));

    assert!(matches!(status, JourneyStatus::Succeeded { .. }));
    let causes: Vec<_> = events.iter().filter_map(retry_cause).collect();
    assert_eq!(causes, [RetryCause::AbnormalTermination]);
}

#[rstest]
#[case::on_step_retry(
    attempting(&[timed_out]).retry_budget(1),
    StepHook::OnStepRetry,
    "step_failed"
)]
#[case::on_step_abnormal_termination(
    attempting(&[crashed]).retry_budget(1).abnormal_termination_retriable(),
    StepHook::OnStepAbnormalTermination,
    "step_abnormal_termination"
)]
fn fail_workflow_from_a_hook_before_the_decision_gives_the_step_up_without_on_step_failure(
    #[case] step: StepDescriptor<Orders>,
    #[case] hook: StepHook,
    #[case] fact: &str,
) {
    let hooked = travel_hooked(step, None, hook, fail_workflow());

    failed_by_the_hook(&hooked, hook);
    assert_eq!(
        kinds(&hooked.events),
        [
            "journey_started",
            "attempt_started",
            fact,
            "hook_called",
            "step_given_up",
            "journey_failed"
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
            "journey_started",
            "attempt_started",
            "step_failed",
            "step_given_up",
            "hook_called",
            "journey_failed"
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
#[case::a_retriable_failure_with_budget_left(
    attempting(&[timed_out]).retry_budget(1),
    &[(StepHook::OnStepRetry, 3), (StepHook::OnStepSuccess, 8)]
)]
#[case::a_retriable_failure_with_the_budget_spent(
    attempting(&[timed_out]),
    &[(StepHook::OnStepFailure, 4)]
)]
#[case::a_failure(attempting(&[declined]).retry_budget(1), &[(StepHook::OnStepFailure, 4)])]
#[case::an_abnormal_termination_the_step_does_not_retry(
    attempting(&[crashed]).retry_budget(1),
    &[(StepHook::OnStepAbnormalTermination, 3), (StepHook::OnStepFailure, 5)]
)]
#[case::an_abnormal_termination_the_step_retries(
    attempting(&[crashed]).retry_budget(1).abnormal_termination_retriable(),
    &[
        (StepHook::OnStepAbnormalTermination, 3),
        (StepHook::OnStepRetry, 4),
        (StepHook::OnStepSuccess, 9)
    ]
)]
#[case::an_abnormal_termination_with_the_budget_spent(
    attempting(&[crashed]).abnormal_termination_retriable(),
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
