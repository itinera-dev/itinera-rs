//! Outcomes: how the journey ended, as its result and its event stream say.

use std::str::FromStr;

use cucumber::then;
use itinera::event::{Event, EventBody};
use itinera::executor::Refusal;
use itinera::journey::{Failure, JourneyId, JourneyStatus, LastFailure, StatusKind};
use itinera::step::{Reason, StepName};

use super::{Unmet, expect, holds, journey, result, stream};
use crate::journey::Journey;
use crate::model::{EventKind, ModelError};
use crate::trace::Line;
use crate::world::World;

#[then(expr = "the last event is {string}")]
fn the_last_event_is(world: &mut World, event: String) -> Result<(), Unmet> {
    let kind = EventKind::named(&event)?;
    let (events, _) = stream(world)?;
    expect(
        events.last().is_some_and(|last| kind.is_of(last)),
        format_args!("the last event to be {event}"),
        &events,
    )
}

#[then(expr = "no event was emitted")]
fn no_event_was_emitted(world: &mut World) -> Result<(), Unmet> {
    let (events, _) = stream(world)?;
    expect(events.is_empty(), "no event", &events)
}

#[then(expr = "journey_aborted names no adapter")]
fn journey_aborted_names_no_adapter(world: &mut World) -> Result<(), Unmet> {
    let (events, lines) = stream(world)?;
    expect(
        lines
            .iter()
            .find(is_journey_aborted)
            .is_some_and(Line::names_no_adapter),
        "a journey_aborted naming no adapter",
        &events,
    )
}

fn is_journey_aborted(line: &&Line) -> bool {
    line.event == "journey_aborted"
}

#[then(expr = "the journey succeeded")]
fn the_journey_succeeded(world: &mut World) -> Result<(), Unmet> {
    ended_as(world, StatusKind::Succeeded)
}

#[then(expr = "the journey failed")]
fn the_journey_failed(world: &mut World) -> Result<(), Unmet> {
    ended_as(world, StatusKind::Failed)
}

#[then(expr = "the journey was aborted")]
fn the_journey_was_aborted(world: &mut World) -> Result<(), Unmet> {
    ended_as(world, StatusKind::Aborted)
}

fn ended_as(world: &World, expected: StatusKind) -> Result<(), Unmet> {
    let found = result(world)?.status.kind();
    holds(
        found == expected,
        format_args!("the journey {expected}"),
        format_args!("it {found}"),
    )
}

#[then(expr = "the journey was aborted with the abort reason {string}")]
fn the_journey_was_aborted_with_the_abort_reason(
    world: &mut World,
    reason: String,
) -> Result<(), Unmet> {
    let found = match &result(world)?.status {
        JourneyStatus::Aborted(abort) => abort.reason().to_string(),
        other => other.kind().to_string(),
    };
    holds(
        found == reason,
        format_args!("the journey aborted with \"{reason}\""),
        format_args!("\"{found}\""),
    )
}

#[then(expr = "run returned a result instead of throwing")]
fn run_returned_a_result(world: &mut World) -> Result<(), Unmet> {
    result(world).map(drop)
}

#[then(expr = "run was refused")]
fn run_was_refused(world: &mut World) -> Result<(), Unmet> {
    refusal(world).map(drop)
}

#[then(expr = "run was refused with the message {string}")]
fn run_was_refused_with_the_message(world: &mut World, message: String) -> Result<(), Unmet> {
    let found = refusal(world)?.to_string();
    holds(
        found.contains(&message),
        format_args!("a refusal carrying \"{message}\""),
        format_args!("\"{found}\""),
    )
}

/// Why `run` refused the last journey.
fn refusal(world: &World) -> Result<&Refusal, Unmet> {
    match &journey(world)?.run {
        Err(refusal) => Ok(refusal),
        Ok(result) => Err(Unmet::Expected(format!(
            "run to be refused; the journey {}",
            result.status.kind()
        ))),
    }
}

#[then(expr = "no journey ID was produced")]
fn no_journey_id_was_produced(world: &mut World) -> Result<(), Unmet> {
    let ids: Vec<&JourneyId> = world.journeys.iter().map(journey_id).collect();
    holds(
        world.admission.is_some() && ids.is_empty(),
        "no workflow instance",
        format_args!("the journey IDs {ids:?}"),
    )
}

fn journey_id(journey: &Journey) -> &JourneyId {
    &journey.journey_id
}

#[then(expr = "the journey ID is {string}")]
fn the_journey_id_is(world: &mut World, id: String) -> Result<(), Unmet> {
    is_the_id(&journey(world)?.journey_id, &id)
}

#[then(expr = "the result's journey ID is {string}")]
fn the_results_journey_id_is(world: &mut World, id: String) -> Result<(), Unmet> {
    is_the_id(&result(world)?.journey_id, &id)
}

fn is_the_id(found: &JourneyId, id: &str) -> Result<(), Unmet> {
    let found: &str = found.as_ref();
    holds(
        found == id,
        format_args!("the journey ID \"{id}\""),
        format_args!("\"{found}\""),
    )
}

#[then(expr = "the result's failure reason has the code {string}")]
fn the_results_failure_reason_has_the_code(world: &mut World, code: String) -> Result<(), Unmet> {
    let found = match &result(world)?.status {
        JourneyStatus::Failed { failure, .. } => reason_of(failure).map(Reason::code),
        _ => None,
    };
    holds(
        found == Some(code.as_str()),
        format_args!("a failure whose reason has the code \"{code}\""),
        format_args!("{found:?}"),
    )
}

/// The reason that explains a failure, when a reason does.
fn reason_of(failure: &Failure) -> Option<&Reason> {
    match failure {
        Failure::Failure(reason)
        | Failure::FailWorkflow(reason)
        | Failure::RetriesExhausted(LastFailure::Reason(reason)) => Some(reason),
        _ => None,
    }
}

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
    /// The step's status, read from the journey's events: `aborted` when `journey_aborted` names
    /// it, since the journey was aborted during the step; otherwise the first event that settles
    /// it, or `not executed` when no attempt started; nothing while it was still being attempted.
    fn of(events: &[Event], step: &str) -> Option<Self> {
        events
            .iter()
            .find_map(|event| aborted_during(event, step))
            .or_else(|| settling(events, step))
            .or_else(|| not_executed(events, step))
    }
}

fn settling(events: &[Event], step: &str) -> Option<Status> {
    events.iter().find_map(|event| settled(event, step))
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
    use crate::model::{Attempt, AttemptOutcome, Dispatching, Model, ReporterFailure};

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
    /// journey during the step.
    fn is_aborted_once_it_succeeded(model: &mut Model) {
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
    #[case::during_which_the_journey_was_aborted_once_it_succeeded(
        is_aborted_once_it_succeeded,
        "charge",
        Status::Aborted,
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
