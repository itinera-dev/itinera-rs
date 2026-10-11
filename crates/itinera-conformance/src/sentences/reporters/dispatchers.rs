//! Events and reporters: the dispatchers the executor is given.

use cucumber::given;

use crate::model::{Dispatching, EventKind, Holding, ModelError};
use crate::world::World;

#[given(expr = "the executor uses its default dispatcher")]
fn the_executor_uses_its_default_dispatcher(world: &mut World) -> Result<(), ModelError> {
    dispatches(world, Dispatching::Default)
}

#[given(expr = "the executor is given a dispatcher holding the reporter {string}")]
fn given_a_dispatcher_holding(world: &mut World, reporter: String) -> Result<(), ModelError> {
    holding(world, reporter, Holding::AddsReporters)
}

#[given(
    expr = "the executor is given a dispatcher holding the reporter {string} that ignores added reporters"
)]
fn given_a_dispatcher_that_ignores_added_reporters(
    world: &mut World,
    reporter: String,
) -> Result<(), ModelError> {
    holding(world, reporter, Holding::IgnoresAddedReporters)
}

#[given(
    expr = "the executor is given a dispatcher holding the reporter {string} that throws when a reporter is added"
)]
fn given_a_dispatcher_that_fails_when_adding(
    world: &mut World,
    reporter: String,
) -> Result<(), ModelError> {
    holding(world, reporter, Holding::FailsWhenAdding)
}

#[given(
    expr = "the executor is given a dispatcher holding the reporter {string} that throws when dispatching {string}"
)]
fn given_a_dispatcher_that_fails_dispatching(
    world: &mut World,
    reporter: String,
    event: String,
) -> Result<(), ModelError> {
    let behaviour = Holding::FailsDispatching(EventKind::named(&event)?);
    holding(world, reporter, behaviour)
}

#[given(expr = "the executor is given a dispatcher factory that fails")]
fn given_a_dispatcher_factory_that_fails(world: &mut World) -> Result<(), ModelError> {
    dispatches(world, Dispatching::FailingFactory)
}

fn holding(world: &mut World, reporter: String, behaviour: Holding) -> Result<(), ModelError> {
    dispatches(
        world,
        Dispatching::Holding {
            reporter,
            behaviour,
        },
    )
}

fn dispatches(world: &mut World, dispatching: Dispatching) -> Result<(), ModelError> {
    if world.model.dispatching != Dispatching::Unstated {
        return Err(ModelError::StatedTwice("the dispatcher"));
    }
    world.model.dispatching = dispatching;
    Ok(())
}
