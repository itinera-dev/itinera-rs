//! Outcomes: how the journey ended, as its result and its event stream say.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use cucumber::gherkin::Step as Sentence;
use cucumber::then;
use itinera::error::Error;
use itinera::event::{Event, EventBody};
use itinera::executor::Refusal;
use itinera::journey::{
    Abort, Failure, JourneyId, JourneyResult, JourneyStatus, LastFailure, StatusKind,
};
use itinera::step::{Reason, StepName};
use itinera::value::AnyValue;
use serde_json::Value;
use uuid::{Uuid, Version};

use super::{Names, Unmet, expect, holds, journey, refused, result, stream};
use crate::journey::Journey;
use crate::model::{EventKind, ModelError, Row, json, rows};
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

#[then(expr = "the result's failure has the cause {string}")]
fn the_results_failure_has_the_cause(world: &mut World, cause: String) -> Result<(), Unmet> {
    let found = failure(world)?.map(Failure::cause);
    holds(
        found.is_some_and(|found| is_named(found, &cause)),
        format_args!("a failure with the cause \"{cause}\""),
        format_args!("{found:?}"),
    )
}

#[then(expr = "the result's failure carries the error {string}")]
fn the_results_failure_carries_the_error(world: &mut World, message: String) -> Result<(), Unmet> {
    let found = failure(world)?
        .and_then(error_of_failure)
        .map(Error::to_string);
    holds(
        found.as_ref() == Some(&message),
        format_args!("a failure carrying the error \"{message}\""),
        format_args!("{found:?}"),
    )
}

#[then(expr = "the result's failure carries no error")]
fn the_results_failure_carries_no_error(world: &mut World) -> Result<(), Unmet> {
    let found = failure(world)?;
    holds(
        found.is_some_and(carries_no_error),
        "a failure carrying no error",
        format_args!("{found:?}"),
    )
}

fn carries_no_error(failure: &Failure) -> bool {
    error_of_failure(failure).is_none()
}

#[then(expr = "the result's failure carries no reason")]
fn the_results_failure_carries_no_reason(world: &mut World) -> Result<(), Unmet> {
    let found = failure(world)?;
    holds(
        found.is_some_and(carries_no_reason),
        "a failure carrying no reason",
        format_args!("{found:?}"),
    )
}

fn carries_no_reason(failure: &Failure) -> bool {
    reason_of(failure).is_none()
}

/// Whether a cause or an abort reason displays as the cases name it.
fn is_named(found: impl fmt::Display, name: &str) -> bool {
    found.to_string() == name
}

/// Why the last journey failed, if it failed.
fn failure(world: &World) -> Result<Option<&Failure>, Unmet> {
    Ok(match &result(world)?.status {
        JourneyStatus::Failed { failure, .. } => Some(failure),
        _ => None,
    })
}

/// The error a failure carries, when an abnormal termination ended it.
fn error_of_failure(failure: &Failure) -> Option<&Error> {
    match failure {
        Failure::AbnormalTermination(error)
        | Failure::RetriesExhausted(LastFailure::Error(error)) => Some(error),
        _ => None,
    }
}

#[then(expr = "the result's abort carries the error {string}")]
fn the_results_abort_carries_the_error(world: &mut World, message: String) -> Result<(), Unmet> {
    let found = abort(world)?.and_then(error_of_abort).map(Error::to_string);
    holds(
        found.as_ref() == Some(&message),
        format_args!("an abort carrying the error \"{message}\""),
        format_args!("{found:?}"),
    )
}

#[then(expr = "the result's abort carries no error")]
fn the_results_abort_carries_no_error(world: &mut World) -> Result<(), Unmet> {
    let found = abort(world)?;
    holds(
        found.is_some_and(carries_no_error_on_abort),
        "an abort carrying no error",
        format_args!("{found:?}"),
    )
}

fn carries_no_error_on_abort(abort: &Abort) -> bool {
    error_of_abort(abort).is_none()
}

/// Why the last journey was aborted, if it was.
fn abort(world: &World) -> Result<Option<&Abort>, Unmet> {
    Ok(match &result(world)?.status {
        JourneyStatus::Aborted(abort) => Some(abort),
        _ => None,
    })
}

/// The error an abort carries, when failing custom code caused it.
fn error_of_abort(abort: &Abort) -> Option<&Error> {
    match abort {
        Abort::StepCouldNotBeBuilt(error)
        | Abort::PolicyCouldNotBeBuilt { error, .. }
        | Abort::HookFailed(error)
        | Abort::ReporterFailed(error) => Some(error),
        _ => None,
    }
}

#[then(expr = "the result names the failed step {string} with the code {string}")]
fn the_result_names_the_failed_step(
    world: &mut World,
    step_name: String,
    code: String,
) -> Result<(), Unmet> {
    let found = failure(world)?.and_then(reason_of).map(Reason::code);
    let (events, lines) = stream(world)?;
    expect(
        found == Some(code.as_str()) && lines.iter().any(|line| fails_at(line, &step_name)),
        format_args!(
            "a failure with the code \"{code}\", whose journey_failed names \"{step_name}\""
        ),
        &events,
    )
}

/// Whether the line is a `journey_failed` naming the step.
pub(super) fn fails_at(line: &Line, step: &str) -> bool {
    line.event == "journey_failed" && line.has_cell("step", step)
}

#[then(
    expr = "the result names the step {string} as where the journey was aborted, with the abort reason {string}"
)]
fn the_result_names_the_step_where_the_journey_was_aborted(
    world: &mut World,
    step_name: String,
    reason: String,
) -> Result<(), Unmet> {
    let found = abort(world)?.map(Abort::reason);
    let (events, _) = stream(world)?;
    expect(
        found.is_some_and(|found| is_named(found, &reason))
            && events.iter().any(|event| is_aborted_at(event, &step_name)),
        format_args!("an abort \"{reason}\", whose journey_aborted names \"{step_name}\""),
        &events,
    )
}

fn is_aborted_at(event: &Event, step: &str) -> bool {
    aborted_during(event, step).is_some()
}

#[then(expr = "the journey ID is a UUID v4")]
fn the_journey_id_is_a_uuid_v4(world: &mut World) -> Result<(), Unmet> {
    let id: &str = journey(world)?.journey_id.as_ref();
    holds(
        Uuid::parse_str(id).is_ok_and(is_random),
        "a UUID v4",
        format_args!("\"{id}\""),
    )
}

fn is_random(uuid: Uuid) -> bool {
    uuid.get_version() == Some(Version::Random)
}

#[then(expr = "the result's data bag contains:")]
fn the_results_data_bag_contains(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    contains(result(world)?, sentence)
}

#[then(expr = "the second journey's data bag contains:")]
fn the_second_journeys_data_bag_contains(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let second = world
        .journeys
        .get(1)
        .ok_or_else(|| Unmet::Case("no second journey ran".to_owned()))?;
    contains(second.run.as_ref().map_err(refused)?, sentence)
}

/// Holds when the result's data bag has every entry of the sentence's table.
fn contains(result: &JourneyResult, sentence: &Sentence) -> Result<(), Unmet> {
    let found = in_json(result)?;
    rows(sentence)?
        .iter()
        .try_for_each(|row| holds_entry(&found, row))
}

fn holds_entry(found: &BTreeMap<&str, Value>, row: &Row) -> Result<(), Unmet> {
    let key = row.required("key")?;
    let value = json(row.required("value")?)?;
    holds(
        found.get(key) == Some(&value),
        format_args!("\"{key}\" = {value} in the data bag"),
        format_args!("{found:?}"),
    )
}

/// The result's data bag, each value as JSON.
fn in_json(result: &JourneyResult) -> Result<BTreeMap<&str, Value>, Unmet> {
    let data = result.status.data().ok_or_else(|| no_data_bag(result))?;
    data.into_iter()
        .map(entry)
        .collect::<Result<_, _>>()
        .map_err(not_json)
}

fn no_data_bag(result: &JourneyResult) -> Unmet {
    Unmet::Expected(format!("a data bag; the journey {}", result.status.kind()))
}

fn entry<'d>((key, value): (&'d String, &AnyValue)) -> Result<(&'d str, Value), serde_json::Error> {
    Ok((key, serde_json::to_value(value)?))
}

fn not_json(error: serde_json::Error) -> Unmet {
    Unmet::Case(error.to_string())
}

#[then(expr = "the result's data bag has no key {string}")]
fn the_results_data_bag_has_no_key(world: &mut World, key: String) -> Result<(), Unmet> {
    let found = in_json(result(world)?)?;
    holds(
        !found.contains_key(key.as_str()),
        format_args!("no \"{key}\" in the data bag"),
        format_args!("{found:?}"),
    )
}

/// The status of an aborted journey has no data bag to read, so the result offers none.
#[then(expr = "the result carries no data bag")]
fn the_result_carries_no_data_bag(world: &mut World) -> Result<(), Unmet> {
    let status = &result(world)?.status;
    holds(
        status.data().is_none(),
        "no data bag",
        format_args!("the data bag of a journey that {}", status.kind()),
    )
}

#[then(expr = "no contribution of {string} was committed")]
fn no_contribution_was_committed(world: &mut World, key: String) -> Result<(), Unmet> {
    let (events, lines) = stream(world)?;
    expect(
        !lines.iter().any(|line| commits(line, &key)),
        format_args!("no contribution_committed of \"{key}\""),
        &events,
    )
}

fn commits(line: &Line, key: &str) -> bool {
    line.event == "contribution_committed" && line.has_cell("key", key)
}

/// `journey_started` holds the keys only, never their values.
#[then(expr = "the journey_started event lists the initial keys {names} without their values")]
fn the_journey_started_event_lists_the_initial_keys(
    world: &mut World,
    keys: Names,
) -> Result<(), Unmet> {
    let mut expected: Vec<String> = keys.into();
    expected.sort();
    let (events, _) = stream(world)?;
    let found = events.iter().find_map(initial_keys);
    expect(
        found == Some(expected.clone()),
        format_args!("journey_started to list {expected:?}"),
        &events,
    )
}

/// The initial keys of a `journey_started`, in order.
fn initial_keys(event: &Event) -> Option<Vec<String>> {
    match &event.body {
        EventBody::JourneyStarted { initial_keys, .. } => {
            let mut keys = initial_keys.clone();
            keys.sort();
            Some(keys)
        }
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
