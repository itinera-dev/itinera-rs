//! The checks a scripted run makes before its journey starts: that the script covers everything
//! the scripted steps can do.

use std::num::NonZeroU32;
use std::ops::ControlFlow;

use super::target::{Kind, Target};
use crate::mode::Mode;
use crate::step::{Attempt, AttemptScript, Ending, StepDescriptor, StepName};
use crate::workflow::WorkflowDescriptor;

/// Checks that each scripted step covers every attempt it can have, that the rescue step of each
/// scripted step that can fail on attempt 1 is scripted, and, when `all` is set, that every step
/// of the workflow is scripted.
///
/// # Panics
///
/// Listing everything the script leaves out, when it leaves anything out.
#[expect(
    clippy::panic,
    reason = "a script that leaves out what a scripted step can do is a mistake in a test, found before its journey"
)]
pub(super) fn check<W: Send + Sync + 'static, M: Mode>(
    descriptor: &WorkflowDescriptor<W, M>,
    targets: &[Target],
    all: bool,
) {
    let required = if all { descriptor.steps() } else { &[] };
    let unscripted_steps = required
        .iter()
        .map(StepDescriptor::name)
        .filter(|step| unscripted(targets, step))
        .map(every_step_required);
    let missing: Vec<String> = targets
        .iter()
        .filter_map(first_uncovered)
        .chain(
            targets
                .iter()
                .filter_map(|target| unscripted_rescue(targets, target)),
        )
        .chain(unscripted_steps)
        .collect();
    if !missing.is_empty() {
        panic!("the script leaves out:\n- {}", missing.join("\n- "));
    }
}

/// The first attempt the step can have that its script leaves out, if any.
fn first_uncovered(target: &Target) -> Option<String> {
    let first = (1..=target.most_attempts().get())
        .filter_map(NonZeroU32::new)
        .map(Attempt::new)
        .try_for_each(|attempt| covering(target, attempt));
    first.break_value().flatten()
}

/// Goes on when the script covers the attempt and the step can have another after it; stops
/// with what is left out when it does not cover it, and with nothing when no attempt can follow.
fn covering(target: &Target, attempt: Attempt) -> ControlFlow<Option<String>> {
    match target.script.attempt(attempt) {
        None => ControlFlow::Break(Some(format!("attempt {attempt} of {}", target.name))),
        Some(scripted) if continues(target.kind, scripted, attempt) => ControlFlow::Continue(()),
        Some(_) => ControlFlow::Break(None),
    }
}

/// Whether the step may be attempted again after this attempt, budget aside.
fn continues(kind: Kind, scripted: &AttemptScript, attempt: Attempt) -> bool {
    let rescued_first = attempt.is_first()
        && matches!(
            kind,
            Kind::Step {
                rescue: Some(_),
                ..
            }
        );
    let retried_abnormally = matches!(
        kind,
        Kind::Step {
            retry_on_abnormal_termination: true,
            ..
        }
    );
    match scripted {
        AttemptScript::RealRun => true,
        AttemptScript::Played(played) => match played.ending {
            Ending::Success | Ending::Skipped(_) => false,
            Ending::Failure(_) => true,
            Ending::AbnormalTermination(_) => rescued_first || retried_abnormally,
        },
    }
}

/// The step's rescue step, when the step's script can fail on attempt 1 and leaves it unscripted.
fn unscripted_rescue(targets: &[Target], target: &Target) -> Option<String> {
    let Kind::Step {
        rescue: Some(rescue),
        ..
    } = target.kind
    else {
        return None;
    };
    let can_fail = target.script.attempt(Attempt::FIRST).is_some_and(can_fail);
    if can_fail && unscripted(targets, &rescue) {
        Some(format!(
            "{rescue}, since {} can fail on attempt 1",
            target.name
        ))
    } else {
        None
    }
}

fn can_fail(scripted: &AttemptScript) -> bool {
    match scripted {
        AttemptScript::RealRun => true,
        AttemptScript::Played(played) => {
            !matches!(played.ending, Ending::Success | Ending::Skipped(_))
        }
    }
}

fn unscripted(targets: &[Target], step: &StepName) -> bool {
    !targets.iter().any(|target| target.is(*step))
}

fn every_step_required(step: StepName) -> String {
    format!("{step}, since every step must be scripted")
}

#[cfg(test)]
mod tests {
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
}
