//! Events and reporters.

mod carried;
mod dispatchers;
mod emitted;

use std::num::NonZeroU64;

use cucumber::{given, then};
use itinera::event::Event;

use super::{Names, Unmet, expect, numbered};
use crate::model::{EventKind, ModelError, ReporterFailure};
use crate::trace::Line;
use crate::world::World;

#[given(expr = "the workflow lists the reporters {names}")]
fn the_workflow_lists_the_reporters(world: &mut World, reporters: Names) -> Result<(), ModelError> {
    world.model.workflow_mut()?.reporters.extend(reporters);
    Ok(())
}

#[given(expr = "the reporter {string} throws")]
fn the_reporter_throws(world: &mut World, reporter: String) -> Result<(), ModelError> {
    fails(world, reporter, ReporterFailure::First)
}

#[given(expr = "the reporter {string} throws on {string}")]
fn the_reporter_throws_on(
    world: &mut World,
    reporter: String,
    event: String,
) -> Result<(), ModelError> {
    let failure = ReporterFailure::On(EventKind::named(&event)?, None);
    fails(world, reporter, failure)
}

#[given(expr = "the reporter {string} fails with {string} on {string}")]
fn the_reporter_fails_with_on(
    world: &mut World,
    reporter: String,
    message: String,
    event: String,
) -> Result<(), ModelError> {
    let failure = ReporterFailure::On(EventKind::named(&event)?, Some(message));
    fails(world, reporter, failure)
}

fn fails(world: &mut World, reporter: String, failure: ReporterFailure) -> Result<(), ModelError> {
    if world.model.reporter_failures.contains_key(&reporter) {
        return Err(ModelError::StatedTwice("how the reporter fails"));
    }
    world.model.reporter_failures.insert(reporter, failure);
    Ok(())
}

#[then(expr = "the reporters {names} received the same events")]
fn the_reporters_received_the_same_events(
    world: &mut World,
    reporters: Names,
) -> Result<(), Unmet> {
    let mut received = reporters
        .as_ref()
        .iter()
        .map(|name| received_by(world, name));
    let Some(first) = received.next().transpose()? else {
        return Ok(());
    };
    received.try_for_each(|other| received_the_same(other, &first))
}

/// Holds when the other reporter's events could be read and are the same as the first's.
fn received_the_same(
    other: Result<Received<'_>, Unmet>,
    first: &Received<'_>,
) -> Result<(), Unmet> {
    let (name, events, lines) = other?;
    expect(
        lines == first.2,
        format_args!("\"{name}\" to receive the events \"{}\" received", first.0),
        &events,
    )
}

/// What the named reporter received: its events, and each one's sequence number and line.
fn received_by<'a>(world: &World, name: &'a str) -> Result<Received<'a>, Unmet> {
    let events = world.recorders.get(name)?.events();
    let lines = events.iter().map(numbered).collect();
    Ok((name, events, lines))
}

type Received<'a> = (&'a str, Vec<Event>, Vec<(NonZeroU64, Line)>);

#[then(expr = "the reporter {string} received the event {string}")]
fn the_reporter_received_the_event(
    world: &mut World,
    reporter: String,
    event: String,
) -> Result<(), Unmet> {
    received(world, &reporter, &event, Times::AtLeastOnce)
}

#[then(expr = "the reporter {string} received the event {string} {int} times")]
fn the_reporter_received_the_event_times(
    world: &mut World,
    reporter: String,
    event: String,
    times: usize,
) -> Result<(), Unmet> {
    received(world, &reporter, &event, Times::Exactly(times))
}

#[then(expr = "the reporter {string} did not receive the event {string}")]
fn the_reporter_did_not_receive_the_event(
    world: &mut World,
    reporter: String,
    event: String,
) -> Result<(), Unmet> {
    received(world, &reporter, &event, Times::Never)
}

fn received(world: &World, reporter: &str, event: &str, times: Times) -> Result<(), Unmet> {
    let kind = EventKind::named(event)?;
    let events = world.recorders.get(reporter)?.events();
    let count = events
        .iter()
        .filter(|received| kind.is_of(received))
        .count();
    expect(
        times.is_met_by(count),
        format_args!("\"{reporter}\" to receive {event} {times}"),
        &events,
    )
}

/// How often a reporter is expected to receive an event.
#[derive(Clone, Copy, Debug, derive_more::Display)]
enum Times {
    #[display("at least once")]
    AtLeastOnce,
    #[display("{_0} times")]
    Exactly(usize),
    #[display("never")]
    Never,
}

impl Times {
    fn is_met_by(self, count: usize) -> bool {
        match self {
            Self::AtLeastOnce => count > 0,
            Self::Exactly(times) => count == times,
            Self::Never => count == 0,
        }
    }
}

#[then(expr = "the reporter {string} received no event")]
fn the_reporter_received_no_event(world: &mut World, reporter: String) -> Result<(), Unmet> {
    let events = world.recorders.get(&reporter)?.events();
    expect(
        events.is_empty(),
        format_args!("\"{reporter}\" to receive no event"),
        &events,
    )
}
