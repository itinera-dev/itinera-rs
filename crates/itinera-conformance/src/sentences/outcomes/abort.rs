//! Outcomes: why the journey was aborted, as its result says.

use cucumber::then;
use itinera::error::Error;
use itinera::journey::{Abort, JourneyStatus};

use super::is_named;
use super::step_status::is_aborted_at;
use crate::sentences::{Unmet, expect, holds, result, stream};
use crate::world::World;

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
