//! The tables of checks, each row standing for one Then sentence of the catalogue, checked in
//! the order written.

use std::num::NonZeroU32;

use cucumber::gherkin::Step as Sentence;
use cucumber::then;

use super::Unmet;
use super::hooks::{the_hook_was_called_times, the_hook_was_not_called};
use super::outcomes::{Status, step_ended_as_aborted, step_ended_as_after_attempts};
use super::received::{
    step_was_built_for_attempt_with_input, step_was_built_with_input,
    step_was_built_with_input_absent, step_was_given_the_service,
    the_hook_received_no_failure_reason, the_hook_received_the_attempt_number,
    the_hook_received_the_cause, the_hook_received_the_error_with_the_message,
    the_hook_received_the_journey_id, the_hook_received_the_step_name,
};
use super::reporters::{
    the_reporter_did_not_receive_the_event, the_reporter_received_the_event,
    the_reporter_received_the_event_times,
};
use crate::model::{ModelError, Row, rows};
use crate::world::World;

const STEP_COLUMNS: [&str; 3] = ["step", "status", "attempts"];

const SERVICE_COLUMNS: [&str; 4] = ["step", "service", "label", "attempt"];

const BUILD_COLUMNS: [&str; 4] = ["step", "input", "value", "attempt"];

const HOOK_COLUMNS: [&str; 9] = [
    "policy",
    "hook",
    "calls",
    "attempt number",
    "step name",
    "journey ID",
    "cause",
    "error",
    "failure reason",
];

const REPORTER_COLUMNS: [&str; 3] = ["reporter", "event", "received"];

/// Checks each row of the sentence's table in turn, stopping at the first that does not hold.
fn each(
    world: &mut World,
    sentence: &Sentence,
    columns: &[&str],
    check: fn(&mut World, &Row) -> Result<(), Unmet>,
) -> Result<(), Unmet> {
    rows(sentence)?
        .iter()
        .try_for_each(|row| checked(world, row, columns, check))
}

fn checked(
    world: &mut World,
    row: &Row,
    columns: &[&str],
    check: fn(&mut World, &Row) -> Result<(), Unmet>,
) -> Result<(), Unmet> {
    row.only(columns)?;
    check(world, row)
}

fn owned(row: &Row, column: &'static str) -> Result<String, ModelError> {
    row.required(column).map(str::to_owned)
}

#[then(expr = "the steps ended as:")]
fn the_steps_ended_as(world: &mut World, #[step] sentence: &Sentence) -> Result<(), Unmet> {
    each(world, sentence, &STEP_COLUMNS, step_ended)
}

fn step_ended(world: &mut World, row: &Row) -> Result<(), Unmet> {
    let step = owned(row, "step")?;
    let status: Status = row.parse("status")?;
    match row.optional("attempts") {
        Some(_) => step_ended_as_after_attempts(world, step, status, row.parse("attempts")?),
        None if status == Status::Aborted => step_ended_as_aborted(world, step),
        None => Err(ModelError::MissingCell("attempts").into()),
    }
}

#[then(expr = "the steps were given the services:")]
fn the_steps_were_given_the_services(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    each(world, sentence, &SERVICE_COLUMNS, step_given)
}

fn step_given(world: &mut World, row: &Row) -> Result<(), Unmet> {
    step_was_given_the_service(
        world,
        owned(row, "step")?,
        owned(row, "service")?,
        owned(row, "label")?,
        row.parse("attempt")?,
    )
}

#[then(expr = "the steps were built with:")]
fn the_steps_were_built_with(world: &mut World, #[step] sentence: &Sentence) -> Result<(), Unmet> {
    each(world, sentence, &BUILD_COLUMNS, step_built)
}

fn step_built(world: &mut World, row: &Row) -> Result<(), Unmet> {
    let step = owned(row, "step")?;
    let input = owned(row, "input")?;
    let value = owned(row, "value")?;
    match (row.optional("attempt"), value.as_str()) {
        (None, "absent") => step_was_built_with_input_absent(world, step, input),
        (None, _) => step_was_built_with_input(world, step, input, value),
        (Some(_), "absent") => Err(ModelError::Cell("value", value).into()),
        (Some(_), _) => {
            let attempt = row.parse("attempt")?;
            step_was_built_for_attempt_with_input(world, step, attempt, input, value)
        }
    }
}

#[then(expr = "the hooks received:")]
fn the_hooks_received(world: &mut World, #[step] sentence: &Sentence) -> Result<(), Unmet> {
    each(world, sentence, &HOOK_COLUMNS, hook_received)
}

/// Checks each filled cell of the row, in the order of the columns.
fn hook_received(world: &mut World, row: &Row) -> Result<(), Unmet> {
    let hook = || owned(row, "hook");
    let policy = || owned(row, "policy");
    match row.optional("calls") {
        Some("0") => the_hook_was_not_called(world, hook()?, policy()?)?,
        Some(_) => the_hook_was_called_times(world, hook()?, policy()?, row.parse("calls")?)?,
        None => {}
    }
    if row.optional("attempt number").is_some() {
        let attempt: NonZeroU32 = row.parse("attempt number")?;
        the_hook_received_the_attempt_number(world, hook()?, policy()?, attempt)?;
    }
    if let Some(step) = row.optional("step name") {
        the_hook_received_the_step_name(world, hook()?, policy()?, step.to_owned())?;
    }
    if let Some(id) = row.optional("journey ID") {
        the_hook_received_the_journey_id(world, hook()?, policy()?, id.to_owned())?;
    }
    if let Some(cause) = row.optional("cause") {
        the_hook_received_the_cause(world, hook()?, policy()?, cause.to_owned())?;
    }
    if let Some(message) = row.optional("error") {
        let message = message.to_owned();
        the_hook_received_the_error_with_the_message(world, hook()?, policy()?, message)?;
    }
    match row.optional("failure reason") {
        Some("none") => the_hook_received_no_failure_reason(world, hook()?, policy()?),
        Some(other) => Err(ModelError::Cell("failure reason", other.to_owned()).into()),
        None => Ok(()),
    }
}

#[then(expr = "the reporters received:")]
fn the_reporters_received(world: &mut World, #[step] sentence: &Sentence) -> Result<(), Unmet> {
    each(world, sentence, &REPORTER_COLUMNS, reporter_received)
}

fn reporter_received(world: &mut World, row: &Row) -> Result<(), Unmet> {
    let reporter = owned(row, "reporter")?;
    let event = owned(row, "event")?;
    match row.required("received")? {
        "yes" => the_reporter_received_the_event(world, reporter, event),
        "no" => the_reporter_did_not_receive_the_event(world, reporter, event),
        _ => {
            let times = row.parse("received")?;
            the_reporter_received_the_event_times(world, reporter, event, times)
        }
    }
}
