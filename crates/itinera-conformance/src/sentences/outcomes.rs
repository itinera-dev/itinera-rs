//! Outcomes read from the event stream alone.

use cucumber::then;

use super::{Unmet, expect, stream};
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
