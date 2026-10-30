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
        .filter_map(|target| first_uncovered(targets, target))
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
fn first_uncovered(targets: &[Target], target: &Target) -> Option<String> {
    let rescue_may_succeed = rescue_may_succeed(targets, target);
    let first = (1..=target.most_attempts().get())
        .filter_map(NonZeroU32::new)
        .map(Attempt::new)
        .try_for_each(|attempt| covering(target, attempt, rescue_may_succeed));
    first.break_value().flatten()
}

/// Goes on when the script covers the attempt and the step can have another after it; stops
/// with what is left out when it does not cover it, and with nothing when no attempt can follow.
fn covering(
    target: &Target,
    attempt: Attempt,
    rescue_may_succeed: bool,
) -> ControlFlow<Option<String>> {
    match target.script.attempt(attempt) {
        None => ControlFlow::Break(Some(format!("attempt {attempt} of {}", target.name))),
        Some(scripted) if continues(target.kind, scripted, attempt, rescue_may_succeed) => {
            ControlFlow::Continue(())
        }
        Some(_) => ControlFlow::Break(None),
    }
}

/// Whether the step may be attempted again after this attempt, budget aside. After a failed
/// attempt 1 of a step with a rescue step, only a rescue step that may succeed lets it run again,
/// since a failed rescue ends the journey.
fn continues(
    kind: Kind,
    scripted: &AttemptScript,
    attempt: Attempt,
    rescue_may_succeed: bool,
) -> bool {
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
        AttemptScript::RealRun => !rescued_first || rescue_may_succeed,
        AttemptScript::Played(played) => match played.ending {
            Ending::Success | Ending::Skipped(_) => false,
            Ending::Failure(_) if rescued_first => rescue_may_succeed,
            Ending::Failure(_) => true,
            Ending::AbnormalTermination(_) if rescued_first => rescue_may_succeed,
            Ending::AbnormalTermination(_) => retried_abnormally,
        },
    }
}

/// Whether the step's rescue step may succeed: unless the script says it fails or terminates
/// abnormally. A rescue step the script leaves out, or runs for real, may.
fn rescue_may_succeed(targets: &[Target], target: &Target) -> bool {
    let Kind::Step {
        rescue: Some(rescue),
        ..
    } = target.kind
    else {
        return true;
    };
    targets
        .iter()
        .find(|other| other.is(rescue))
        .and_then(|scripted| scripted.script.attempt(Attempt::FIRST))
        .is_none_or(may_succeed)
}

fn may_succeed(scripted: &AttemptScript) -> bool {
    match scripted {
        AttemptScript::RealRun => true,
        AttemptScript::Played(played) => matches!(played.ending, Ending::Success),
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
mod tests;
