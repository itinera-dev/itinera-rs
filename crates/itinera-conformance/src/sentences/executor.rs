//! The executor.

use cucumber::{then, when};

use super::{Unmet, expect, holds, is_of, numbered, of_journey};
use crate::journey::{self, Journey, Unrunnable};
use crate::world::World;

#[when(expr = "the same executor runs the workflow twice, with a new instance each time")]
async fn the_same_executor_runs_the_workflow_twice(world: &mut World) -> Result<(), Unrunnable> {
    journey::run(world, 2).await
}

#[then(expr = "the two journeys have different journey IDs")]
fn the_two_journeys_have_different_journey_ids(world: &mut World) -> Result<(), Unmet> {
    let (first, second) = two(world)?;
    holds(
        first.journey_id != second.journey_id,
        "two journey IDs",
        format_args!("\"{}\" twice", first.journey_id),
    )
}

#[then(
    expr = "the second journey's events are the first journey's events, apart from the journey ID and timestamps"
)]
fn the_second_journeys_events_are_the_firsts(world: &mut World) -> Result<(), Unmet> {
    let (first, second) = two(world)?;
    let events = world.recorders.stream(&world.model)?.events();
    let firsts = of_journey(events.clone(), &first.journey_id);
    let seconds = of_journey(events, &second.journey_id);
    let numbered_firsts: Vec<_> = firsts.iter().map(numbered).collect();
    let numbered_seconds: Vec<_> = seconds.iter().map(numbered).collect();
    expect(
        !firsts.is_empty() && numbered_firsts == numbered_seconds,
        format_args!("the second journey's events to be the first's, {firsts:?}"),
        &seconds,
    )
}

/// The scenario's two journeys, in the order they ran.
fn two(world: &World) -> Result<(&Journey, &Journey), Unmet> {
    match world.journeys.as_slice() {
        [first, second] => Ok((first, second)),
        journeys => Err(Unmet::Case(format!(
            "the workflow ran {} journeys, not two",
            journeys.len()
        ))),
    }
}

#[then(expr = "each instance's reporter {string} received only its own journey's events")]
fn each_instances_reporter_received_only_its_own_journeys_events(
    world: &mut World,
    reporter: String,
) -> Result<(), Unmet> {
    if world.journeys.is_empty() {
        return Err(Unmet::Case("no journey ran".to_owned()));
    }
    world
        .journeys
        .iter()
        .try_for_each(|journey| received_only_its_own(journey, &reporter))
}

/// Holds when the journey's own reporter of this name received events, all of the journey.
fn received_only_its_own(journey: &Journey, reporter: &str) -> Result<(), Unmet> {
    let events = journey
        .reporters
        .get(reporter)
        .ok_or_else(|| Unmet::Case(format!("the workflow lists no reporter \"{reporter}\"")))?
        .events();
    let own = events.iter().all(|event| is_of(event, &journey.journey_id));
    expect(
        !events.is_empty() && own,
        format_args!(
            "\"{reporter}\" of journey \"{}\" to receive its journey's events only",
            journey.journey_id
        ),
        &events,
    )
}

#[then(expr = "the reporter {string} was made with the journey ID {string}")]
fn the_reporter_was_made_with_the_journey_id(
    world: &mut World,
    reporter: String,
    id: String,
) -> Result<(), Unmet> {
    let made_with = world.recorders.get(&reporter)?.journey_id();
    let found = made_with.as_ref().map(AsRef::<str>::as_ref);
    holds(
        found == Some(id.as_str()),
        format_args!("\"{reporter}\" made with the journey ID \"{id}\""),
        format_args!("{found:?}"),
    )
}
