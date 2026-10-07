//! Journey IDs.

use cucumber::given;

use crate::model::{IdGenerator, ModelError};
use crate::world::World;

#[given(expr = "the workflow has no ID generator")]
fn the_workflow_has_no_id_generator(world: &mut World) -> Result<(), ModelError> {
    generates(world, IdGenerator::Default)
}

#[given(expr = "the workflow's ID generator returns {string}")]
fn the_id_generator_returns(world: &mut World, id: String) -> Result<(), ModelError> {
    generates(world, IdGenerator::Returns(id))
}

fn generates(world: &mut World, generator: IdGenerator) -> Result<(), ModelError> {
    world.model.workflow_mut()?.id_generator = generator;
    Ok(())
}
