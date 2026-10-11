//! Outcomes: why the journey failed, as its result says.

use cucumber::then;
use itinera::error::Error;
use itinera::journey::{Failure, JourneyStatus, LastFailure};
use itinera::step::Reason;

use super::is_named;
use crate::sentences::{Unmet, expect, holds, result, stream};
use crate::trace::Line;
use crate::world::World;

#[then(expr = "the result's failure reason has the code {string}")]
fn the_results_failure_reason_has_the_code(world: &mut World, code: String) -> Result<(), Unmet> {
    let found = failure(world)?.and_then(reason_of).map(Reason::code);
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
pub(crate) fn fails_at(line: &Line, step: &str) -> bool {
    line.event == "journey_failed" && line.has_cell("step", step)
}
