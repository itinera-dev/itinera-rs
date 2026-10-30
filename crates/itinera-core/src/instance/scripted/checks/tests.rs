use rstest::rstest;

use super::super::fixtures::{
    Charge, Charging, Orderly, OrderlyStep, not_retried_abnormally, rescued, retried,
};
use crate::journey::StatusKind;

fn charge(scripted: Orderly) -> Charge {
    scripted.step("charge")
}

fn succeeds(attempt: Charging) -> OrderlyStep {
    attempt.will_succeed()
}

fn skips(attempt: Charging) -> OrderlyStep {
    attempt.will_skip()
}

fn terminates_abnormally(attempt: Charging) -> OrderlyStep {
    attempt.will_terminate_abnormally("gateway down")
}

#[test]
#[should_panic(expected = "the script leaves out:\n- attempt 3 of charge")]
fn a_scripted_step_must_cover_every_attempt_its_retry_budget_allows() {
    drop(
        charge(retried(2))
            .will_fail("timeout")
            .on_attempt(2)
            .will_fail("timeout")
            .run(),
    );
}

#[rstest]
#[case::a_success(succeeds, StatusKind::Succeeded)]
#[case::a_skip(skips, StatusKind::Succeeded)]
#[case::an_abnormal_termination_not_retried(terminates_abnormally, StatusKind::Failed)]
fn no_attempt_is_needed_after_one_the_step_cannot_be_attempted_again_after(
    #[case] ending: fn(Charging) -> OrderlyStep,
    #[case] status: StatusKind,
) {
    let charge = charge(not_retried_abnormally(3))
        .will_fail("timeout")
        .on_attempt(2);

    let run = ending(charge).run().unwrap();

    assert_eq!(run.result.status.kind(), status);
}

#[test]
#[should_panic(expected = "the script leaves out:\n- attempt 2 of charge")]
fn a_failure_with_budget_left_needs_the_next_attempt_scripted() {
    drop(charge(retried(1)).will_fail("declined").run());
}

#[test]
#[should_panic(expected = "the script leaves out:\n- attempt 2 of charge")]
fn an_abnormal_termination_the_step_retries_needs_the_next_attempt_scripted() {
    drop(
        retried(1)
            .step("charge")
            .will_terminate_abnormally("gateway down")
            .run(),
    );
}

#[test]
#[should_panic(expected = "the script leaves out:\n- attempt 2 of charge")]
fn an_attempt_that_runs_for_real_may_be_followed_by_another() {
    drop(charge(retried(1)).will_run_for_real().run());
}

#[test]
#[should_panic(
    expected = "the script leaves out:\n- attempt 2 of charge\n- charge_rescue, since charge can fail on attempt 1"
)]
fn a_rescued_step_that_can_fail_on_attempt_1_needs_attempt_2_and_its_rescue_step_scripted() {
    drop(
        charge(rescued())
            .will_terminate_abnormally("gateway down")
            .run(),
    );
}

#[test]
#[should_panic(expected = "charge_rescue, since charge can fail on attempt 1")]
fn a_rescued_step_whose_attempt_1_runs_for_real_needs_its_rescue_step_scripted() {
    drop(
        charge(rescued())
            .will_run_for_real()
            .on_attempt(2)
            .will_succeed()
            .run(),
    );
}

#[rstest]
#[case::a_success(rescue_succeeds)]
#[case::a_real_run(rescue_runs_for_real)]
#[should_panic(expected = "the script leaves out:\n- attempt 2 of charge")]
fn a_rescued_step_whose_rescue_step_may_succeed_needs_attempt_2_scripted(
    #[case] rescue_ending: fn(Charge) -> OrderlyStep,
) {
    let rescue_step = charge(rescued())
        .will_fail("declined")
        .and_step("charge_rescue");

    drop(rescue_ending(rescue_step).run());
}

#[rstest]
#[case::a_failure(rescue_fails)]
#[case::an_abnormal_termination(rescue_terminates_abnormally)]
fn a_rescued_step_needs_no_attempt_after_a_rescue_step_that_fails(
    #[case] rescue_ending: fn(Charge) -> OrderlyStep,
) {
    let rescue_step = charge(rescued())
        .will_fail("declined")
        .and_step("charge_rescue");

    let run = rescue_ending(rescue_step).run().unwrap();

    assert_eq!(run.result.status.kind(), StatusKind::Failed);
}

#[rstest]
#[case::a_failure(rescue_fails)]
#[case::an_abnormal_termination(rescue_terminates_abnormally)]
fn a_rescued_step_run_for_real_needs_no_attempt_after_a_rescue_step_that_fails(
    #[case] rescue_ending: fn(Charge) -> OrderlyStep,
) {
    let rescue_step = charge(rescued())
        .will_run_for_real()
        .and_step("charge_rescue");

    let run = rescue_ending(rescue_step).run().unwrap();

    assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
}

#[test]
fn an_attempt_described_after_a_rescue_step_that_fails_is_no_mistake() {
    let run = charge(rescued())
        .will_fail("declined")
        .on_attempt(2)
        .will_succeed()
        .and_step("charge_rescue")
        .will_fail("still declined")
        .run()
        .unwrap();

    assert_eq!(run.result.status.kind(), StatusKind::Failed);
}

fn rescue_succeeds(attempt: Charge) -> OrderlyStep {
    attempt.will_succeed()
}

fn rescue_runs_for_real(attempt: Charge) -> OrderlyStep {
    attempt.will_run_for_real()
}

fn rescue_fails(attempt: Charge) -> OrderlyStep {
    attempt.will_fail("still declined")
}

fn rescue_terminates_abnormally(attempt: Charge) -> OrderlyStep {
    attempt.will_terminate_abnormally("gateway down")
}

#[test]
fn a_rescued_step_that_succeeds_on_attempt_1_needs_no_rescue_step_scripted() {
    let run = charge(rescued()).will_succeed().run().unwrap();

    assert_eq!(run.result.status.kind(), StatusKind::Succeeded);
}

#[test]
#[should_panic(expected = "the script leaves out:\n- ship, since every step must be scripted")]
fn ensuring_all_scripted_requires_every_step_of_the_workflow_to_be_scripted() {
    drop(
        retried(0)
            .ensure_all_scripted()
            .step("charge")
            .will_succeed()
            .run(),
    );
}
