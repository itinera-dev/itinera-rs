use rstest::rstest;

use super::*;
use crate::engine::fixtures::{
    CHARGE, SHIP, attempting, audit, crashed, data_of, declined, fail_workflow, failed_by_the_hook,
    kinds, scripted, skips, succeed, timed_out, travel_hooked, travel_with_amount, travel_workflow,
};
use crate::event::Event;
use crate::journey::JourneyStatus;
use crate::policy::Lifecycle;
use crate::step::{Outcome, StepAttempt, StepDescriptor};
use crate::workflow::fixtures::orders;

#[test]
fn a_skipped_step_discards_its_contributions_and_the_journey_goes_on() {
    let workflow = orders()
        .step(StepDescriptor::new(CHARGE, || Ok(Outcome::skipped())))
        .step(StepDescriptor::new(SHIP, succeed));

    let (status, events) = travel_workflow(workflow);

    assert!(matches!(status, JourneyStatus::Succeeded { .. }));
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "step_skipped",
            "contributions_discarded",
            "attempt_started",
            "step_succeeded",
            "journey_succeeded"
        ]
    );
}

#[test]
fn finish_workflow_from_on_step_success_succeeds_the_journey_without_the_steps_left() {
    let hooked = travel_hooked(
        attempting(&[]),
        Some(StepDescriptor::new(SHIP, crashed)),
        StepHook::OnStepSuccess,
        Lifecycle::FinishWorkflow,
    );

    assert_eq!(data_of(&hooked.status), [("attempt".to_string(), 1)]);
    assert_eq!(
        kinds(&hooked.events),
        [
            "journey_started",
            "attempt_started",
            "step_succeeded",
            "contribution_committed",
            "hook_called",
            "journey_succeeded"
        ]
    );
    assert!(matches!(
        hooked.events.last().map(|e| &e.body),
        Some(EventBody::JourneySucceeded { decided_by: Some(policy) }) if *policy == audit()
    ));
}

#[test]
fn fail_workflow_from_on_step_success_fails_the_journey_after_committing_the_contributions() {
    let hooked = travel_hooked(
        attempting(&[]),
        Some(StepDescriptor::new(SHIP, succeed)),
        StepHook::OnStepSuccess,
        fail_workflow(),
    );

    failed_by_the_hook(&hooked, StepHook::OnStepSuccess);
    assert_eq!(data_of(&hooked.status), [("attempt".to_string(), 1)]);
    assert_eq!(
        kinds(&hooked.events),
        [
            "journey_started",
            "attempt_started",
            "step_succeeded",
            "contribution_committed",
            "hook_called",
            "journey_failed"
        ]
    );
}

fn key_of(event: &Event) -> Option<&str> {
    match &event.body {
        EventBody::ContributionCommitted { key, .. } | EventBody::DataOverwritten { key, .. } => {
            Some(key)
        }
        _ => None,
    }
}

#[test]
fn a_successful_attempts_contributions_are_committed_in_order_reporting_what_they_overwrote() {
    let workflow = orders().step(scripted(|contributor, _| {
        contributor.contribute("receipt", 1_i64);
        contributor.contribute("amount", 40_i64);
        contributor.contribute("receipt", 2_i64);
        Ok(Outcome::success())
    }));

    let (status, events) = travel_with_amount(workflow);

    assert_eq!(
        data_of(&status),
        [("amount".to_string(), 40), ("receipt".to_string(), 2)]
    );
    assert_eq!(
        kinds(&events),
        [
            "journey_started",
            "attempt_started",
            "step_succeeded",
            "contribution_committed",
            "contribution_committed",
            "data_overwritten",
            "journey_succeeded"
        ]
    );
    let keys: Vec<_> = events.iter().filter_map(key_of).collect();
    assert_eq!(keys, ["receipt", "amount", "amount"]);
    let Some(EventBody::ContributionCommitted { source, .. }) = events.get(3).map(|e| &e.body)
    else {
        panic!("the fourth event is not contribution_committed");
    };
    assert_eq!(source, &Source::Step(StepAttempt::first(CHARGE)));
}

#[rstest]
#[case::a_skip(skips)]
#[case::a_failure(declined)]
#[case::a_retriable_failure(timed_out)]
#[case::an_abnormal_termination(crashed)]
fn an_attempt_that_does_not_succeed_commits_nothing(#[case] ends: fn() -> Result<Outcome, Error>) {
    let workflow = orders().step(scripted(move |contributor, _| {
        contributor.contribute("receipt", 1_i64);
        ends()
    }));

    let (status, events) = travel_with_amount(workflow);

    assert_eq!(data_of(&status), [("amount".to_string(), 42)]);
    assert!(!kinds(&events).contains(&"contribution_committed"));
}
