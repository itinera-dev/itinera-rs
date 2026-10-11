//! The workflows the macros declare, which a scenario names instead of declaring one.

use cucumber::given;

use crate::fixture::Fixture;
use crate::model::ModelError;
use crate::world::World;

#[given(expr = "the {fixture} workflow")]
fn the_fixture_workflow(world: &mut World, fixture: Fixture) -> Result<(), ModelError> {
    if world.fixture.is_some() || world.model.workflow().is_ok() {
        return Err(ModelError::WorkflowDeclaredTwice);
    }
    world.fixture = Some(fixture);
    Ok(())
}
