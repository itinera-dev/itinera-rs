//! Outcomes: how the journey ended, as its result and its event stream say.

mod abort;
mod data_bag;
mod failure;
mod journey_id;
mod step_status;

use std::fmt;

use cucumber::then;
use itinera::executor::Refusal;
use itinera::journey::{JourneyStatus, StatusKind};

pub(super) use failure::fails_at;

use super::{Unmet, expect, holds, journey, result, stream};
use crate::model::EventKind;
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

/// Whether a cause or an abort reason displays as the cases name it.
fn is_named(found: impl fmt::Display, name: &str) -> bool {
    found.to_string() == name
}
