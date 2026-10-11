//! Outcomes: how each step ended, as the event stream says.

use std::str::FromStr;

use cucumber::then;
use itinera::event::{Event, EventBody};
use itinera::step::StepName;

use crate::model::ModelError;
use crate::sentences::{Unmet, expect, stream};
use crate::world::World;

#[then(expr = "step {string} ended as {status} after {int} attempts")]
fn step_ended_as_after_attempts(
    world: &mut World,
    step_name: String,
    status: Status,
    attempts: usize,
) -> Result<(), Unmet> {
    let (events, _) = stream(world)?;
    let ended = Status::of(&events, &step_name);
    let started = attempts_of(&events, &step_name);
    expect(
        ended == Some(status) && started == attempts,
        format_args!("\"{step_name}\" to end as {status} after {attempts} attempts"),
        &events,
    )
}

#[then(expr = "step {string} ended as aborted")]
fn step_ended_as_aborted(world: &mut World, step_name: String) -> Result<(), Unmet> {
    let (events, _) = stream(world)?;
    expect(
        Status::of(&events, &step_name) == Some(Status::Aborted),
        format_args!("\"{step_name}\" to end as aborted"),
        &events,
    )
}

/// A step's status at the end of the journey, as the cases name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, cucumber::Parameter, derive_more::Display)]
#[param(
    name = "status",
    regex = "succeeded|failed|skipped|not executed|aborted"
)]
enum Status {
    #[display("succeeded")]
    Succeeded,
    #[display("failed")]
    Failed,
    #[display("skipped")]
    Skipped,
    #[display("not executed")]
    NotExecuted,
    #[display("aborted")]
    Aborted,
}

impl FromStr for Status {
    type Err = ModelError;

    fn from_str(name: &str) -> Result<Self, ModelError> {
        Ok(match name {
            "succeeded" => Self::Succeeded,
            "failed" => Self::Failed,
            "skipped" => Self::Skipped,
            "not executed" => Self::NotExecuted,
            "aborted" => Self::Aborted,
            _ => return Err(ModelError::Cell("status", name.to_owned())),
        })
    }
}

impl Status {
    /// The step's status, read from the journey's events: the first event that settles it, which
    /// stays final even when the journey is aborted afterwards; otherwise `aborted` when
    /// `journey_aborted` names it, since the journey was aborted before the step settled, or
    /// `not executed` when no attempt started; nothing while it was still being attempted.
    fn of(events: &[Event], step: &str) -> Option<Self> {
        settling(events, step)
            .or_else(|| aborted_before_settling(events, step))
            .or_else(|| not_executed(events, step))
    }
}

fn settling(events: &[Event], step: &str) -> Option<Status> {
    events.iter().find_map(|event| settled(event, step))
}

fn aborted_before_settling(events: &[Event], step: &str) -> Option<Status> {
    events.iter().find_map(|event| aborted_during(event, step))
}

fn not_executed(events: &[Event], step: &str) -> Option<Status> {
    (attempts_of(events, step) == 0).then_some(Status::NotExecuted)
}

/// `aborted`, when the event is `journey_aborted` and names the step.
fn aborted_during(event: &Event, step: &str) -> Option<Status> {
    match &event.body {
        EventBody::JourneyAborted { abort, .. }
            if abort
                .step()
                .is_some_and(|aborted| is_of_step(aborted, step)) =>
        {
            Some(Status::Aborted)
        }
        _ => None,
    }
}

pub(super) fn is_aborted_at(event: &Event, step: &str) -> bool {
    aborted_during(event, step).is_some()
}

/// The status a step outcome or decision settles the step in, if the event is one of its own.
fn settled(event: &Event, step: &str) -> Option<Status> {
    match &event.body {
        EventBody::StepSucceeded { step: attempt, .. } if is_of_step(attempt.step, step) => {
            Some(Status::Succeeded)
        }
        EventBody::StepSkipped { step: attempt, .. } if is_of_step(attempt.step, step) => {
            Some(Status::Skipped)
        }
        EventBody::StepGivenUp { step: attempt, .. } if is_of_step(attempt.step, step) => {
            Some(Status::Failed)
        }
        _ => None,
    }
}

/// How many attempts of the step started.
fn attempts_of(events: &[Event], step: &str) -> usize {
    events
        .iter()
        .filter(|event| starts_an_attempt_of(event, step))
        .count()
}

fn starts_an_attempt_of(event: &Event, step: &str) -> bool {
    matches!(&event.body, EventBody::AttemptStarted { step: attempt, .. } if is_of_step(attempt.step, step))
}

fn is_of_step(name: StepName, step: &str) -> bool {
    let name: &str = name.as_ref();
    name == step
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::executor::Executor;
    use crate::journey::tests::{attempt, ran, timed_out};
    use crate::model::{Attempt, AttemptOutcome, Dispatching, EventKind, Model, ReporterFailure};

    fn ends(model: &mut Model, step: &str, outcome: AttemptOutcome) {
        model.step_mut(step).unwrap().attempts = Some(vec![attempt(outcome)]);
    }

    fn succeeds(model: &mut Model) {
        ends(model, "charge", AttemptOutcome::Success);
    }

    fn is_given_up(model: &mut Model) {
        ends(model, "charge", timed_out());
    }

    fn skips(model: &mut Model) {
        ends(model, "charge", AttemptOutcome::Skipped(None));
    }

    fn is_retried(model: &mut Model) {
        let charge = model.step_mut("charge").unwrap();
        charge.retries = 1;
        charge.attempts = Some(vec![attempt(timed_out()), attempt(AttemptOutcome::Success)]);
    }

    fn cannot_be_built(model: &mut Model) {
        model.step_mut("charge").unwrap().construction_failure = Some("no connection".to_owned());
    }

    /// The charge step succeeds, and a reporter fails on its contribution, which aborts the
    /// journey once the step has settled.
    fn succeeds_before_a_reporter_aborts_the_journey(model: &mut Model) {
        let charge = model.step_mut("charge").unwrap();
        charge.attempts = Some(vec![Attempt {
            outcome: AttemptOutcome::Success,
            contributes: vec![("receipt".to_owned(), serde_json::json!("R-1"))],
        }]);
        model.dispatching = Dispatching::Default;
        model.workflow_mut().unwrap().reporters = vec!["fragile".to_owned(), "audit".to_owned()];
        let committed = EventKind::named("contribution_committed").unwrap();
        model
            .reporter_failures
            .insert("fragile".to_owned(), ReporterFailure::On(committed, None));
    }

    #[rstest]
    #[case::that_succeeded(succeeds, "charge", Status::Succeeded, 1)]
    #[case::given_up(is_given_up, "charge", Status::Failed, 1)]
    #[case::that_skipped_itself(skips, "charge", Status::Skipped, 1)]
    #[case::that_succeeded_once_retried(is_retried, "charge", Status::Succeeded, 2)]
    #[case::after_one_given_up(is_given_up, "ship", Status::NotExecuted, 0)]
    #[case::that_could_not_be_built(cannot_be_built, "charge", Status::Aborted, 1)]
    #[case::that_succeeded_before_the_journey_was_aborted(
        succeeds_before_a_reporter_aborts_the_journey,
        "charge",
        Status::Succeeded,
        1
    )]
    fn a_steps_status_and_attempts_are_read_from_its_journeys_events(
        #[case] script: fn(&mut Model),
        #[case] step: &str,
        #[case] status: Status,
        #[case] attempts: usize,
    ) {
        let events = ran(Executor::Local, &["charge", "ship"], script);
        assert_eq!(Status::of(&events, step), Some(status));
        assert_eq!(attempts_of(&events, step), attempts);
    }
}
