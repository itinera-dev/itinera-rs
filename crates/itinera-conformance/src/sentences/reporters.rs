//! Events and reporters.

use cucumber::given;

use super::Names;
use crate::model::{
    Dispatching, EventKind, Holding, HookAction, Level, ModelError, ReporterFailure, StepAction,
    json,
};
use crate::world::World;

#[given(expr = "the workflow lists the reporters {names}")]
fn the_workflow_lists_the_reporters(world: &mut World, reporters: Names) -> Result<(), ModelError> {
    world.model.workflow_mut()?.reporters.extend(reporters.0);
    Ok(())
}

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

/// The "emits" sentences of a step with no data or with data written as JSON.
#[given(
    regex = r#"^step "([^"]*)" emits (\w+) "([^"]*)"(?: with data ([\[{"0-9-].*|true|false|null))?$"#
)]
fn step_emits(
    world: &mut World,
    step_name: String,
    kind: String,
    message: String,
    data: String,
) -> Result<(), ModelError> {
    let action = StepAction::Emit {
        level: Level::of("step", &kind)?,
        message,
        data: (!data.is_empty()).then(|| json(&data)).transpose()?,
    };
    world.model.step_mut(&step_name)?.actions.push(action);
    Ok(())
}

#[given(regex = r#"^the hook "([^"]*)" of policy "([^"]*)" emits (\w+) "([^"]*)"$"#)]
fn the_hook_emits(
    world: &mut World,
    hook: String,
    policy: String,
    kind: String,
    message: String,
) -> Result<(), ModelError> {
    let action = HookAction::Emit {
        level: Level::of("journey", &kind)?,
        message,
    };
    world.model.hook_mut(&policy, &hook)?.actions.push(action);
    Ok(())
}
