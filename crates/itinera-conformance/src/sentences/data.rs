//! Data.

use cucumber::gherkin::Step as Sentence;
use cucumber::given;

use crate::model::{ModelError, json, rows};
use crate::world::World;

#[given(expr = "the data bag is empty")]
fn the_data_bag_is_empty(world: &mut World) -> Result<(), ModelError> {
    if world.model.initial_data.is_empty() {
        Ok(())
    } else {
        Err(ModelError::StatedTwice("the initial data"))
    }
}

#[given(expr = "the data bag contains:")]
fn the_data_bag_contains(world: &mut World, #[step] sentence: &Sentence) -> Result<(), ModelError> {
    for row in rows(sentence)? {
        let entry = (
            row.required("key")?.to_owned(),
            json(row.required("value")?)?,
        );
        world.model.initial_data.push(entry);
    }
    Ok(())
}

#[given(expr = "step {string} ignores failures of its emit calls and carries on")]
fn step_ignores_failed_emits(world: &mut World, step_name: String) -> Result<(), ModelError> {
    world.model.step_mut(&step_name)?.ignores_failed_emits = true;
    Ok(())
}
