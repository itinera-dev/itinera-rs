//! Outcomes: the journey ID, as the workflow instance and the result carry it.

use cucumber::then;
use itinera::journey::JourneyId;
use uuid::{Uuid, Version};

use crate::journey::Journey;
use crate::sentences::{Unmet, holds, journey, result};
use crate::world::World;

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
