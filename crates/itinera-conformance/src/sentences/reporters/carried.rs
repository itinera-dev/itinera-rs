//! Events and reporters: what every event carries, and what no event may carry.

use cucumber::then;
use itinera::event::Event;

use crate::model::Level;
use crate::sentences::{Unmet, expect, stream};
use crate::world::World;

#[then(
    expr = "every event carries the journey ID {string} and the workflow name {string}, with increasing sequence numbers"
)]
fn every_event_carries_the_journey_id_and_workflow_name(
    world: &mut World,
    id: String,
    workflow: String,
) -> Result<(), Unmet> {
    let (events, _) = stream(world)?;
    let carried = events.iter().all(|event| belongs_to(event, &id, &workflow));
    let increasing = events.windows(2).all(in_sequence);
    expect(
        !events.is_empty() && carried && increasing,
        format_args!("every event of \"{workflow}\" for \"{id}\", in increasing sequence"),
        &events,
    )
}

/// Whether the event is of this journey of this workflow.
fn belongs_to(event: &Event, id: &str, workflow: &str) -> bool {
    let journey_id: &str = event.journey_id.as_ref();
    let name: &str = event.workflow.as_ref();
    journey_id == id && name == workflow
}

/// Whether two consecutive events have increasing sequence numbers.
fn in_sequence(pair: &[Event]) -> bool {
    matches!(pair, [earlier, later] if earlier.sequence < later.sequence)
}

#[then(expr = "no engine event carries the value of {string}")]
fn no_engine_event_carries_the_value_of(world: &mut World, key: String) -> Result<(), Unmet> {
    let values = world.model.values_of(&key);
    if values.is_empty() {
        return Err(Unmet::Case(format!(
            "the scenario gives \"{key}\" no value"
        )));
    }
    let (events, lines) = stream(world)?;
    let carrying = events
        .iter()
        .zip(&lines)
        .filter(|(event, _)| is_engine_event(event))
        .any(|(_, line)| line.carries_any(&values));
    expect(
        !carrying,
        format_args!("no engine event to carry the value of \"{key}\""),
        &events,
    )
}

/// Whether the engine emitted the event, rather than a step or a hook emitting it.
fn is_engine_event(event: &Event) -> bool {
    !Level::is_emitted(event.kind())
}

#[then(expr = "no event carries the message {string}")]
fn no_event_carries_the_message(world: &mut World, message: String) -> Result<(), Unmet> {
    let (events, lines) = stream(world)?;
    let carrying = lines.iter().any(|line| line.carries_message(&message));
    expect(
        !carrying,
        format_args!("no event to carry \"{message}\""),
        &events,
    )
}
