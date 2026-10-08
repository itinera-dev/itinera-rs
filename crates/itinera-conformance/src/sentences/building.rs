//! Building a workflow.

use cucumber::gherkin::Step as Sentence;
use cucumber::given;

use crate::model::{Attempt, Input, ModelError, Row, StepAction, ValueType, attempts, json, rows};
use crate::world::World;

#[given(expr = "a workflow {string} with the steps:")]
fn a_workflow_with_the_steps(
    world: &mut World,
    #[step] sentence: &Sentence,
    workflow: String,
) -> Result<(), ModelError> {
    let steps = rows(sentence)?
        .iter()
        .map(step_name)
        .collect::<Result<_, _>>()?;
    world.model.declare(workflow, steps)
}

fn step_name(row: &Row) -> Result<String, ModelError> {
    row.required("step").map(str::to_owned)
}

#[given(expr = "step {string} requests input {string} of type {type}")]
fn step_requests_input(
    world: &mut World,
    step_name: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    request(world, &step_name, key, value_type, false)
}

#[given(expr = "step {string} requests optional input {string} of type {type}")]
fn step_requests_optional_input(
    world: &mut World,
    step_name: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    request(world, &step_name, key, value_type, true)
}

fn request(
    world: &mut World,
    step: &str,
    key: String,
    value_type: ValueType,
    optional: bool,
) -> Result<(), ModelError> {
    world.model.step_mut(step)?.inputs.push(Input {
        key,
        value_type,
        optional,
    });
    Ok(())
}

/// Both "contributes" sentences, since one begins as the other does.
#[given(regex = r#"^step "([^"]*)" contributes "([^"]*)" = (.+)$"#)]
fn step_contributes(
    world: &mut World,
    step_name: String,
    key: String,
    rest: String,
) -> Result<(), ModelError> {
    let action = match rest.split_once(" and then changes it to ") {
        Some((value, changed)) => StepAction::ContributeThenChange {
            key,
            value: json(value)?,
            changed: json(changed)?,
        },
        None => StepAction::Contribute {
            key,
            value: json(&rest)?,
        },
    };
    world.model.step_mut(&step_name)?.actions.push(action);
    Ok(())
}

#[given(expr = "step {string} succeeds")]
fn step_succeeds(world: &mut World, step_name: String) -> Result<(), ModelError> {
    ends(world, &step_name, vec![Attempt::success()])
}

#[given(expr = "step {string} attempts:")]
fn step_attempts(
    world: &mut World,
    #[step] sentence: &Sentence,
    step_name: String,
) -> Result<(), ModelError> {
    ends(world, &step_name, attempts(rows(sentence)?)?)
}

fn ends(world: &mut World, step: &str, attempts: Vec<Attempt>) -> Result<(), ModelError> {
    let script = world.model.step_mut(step)?;
    if script.attempts.is_some() {
        return Err(ModelError::StatedTwice("how the step's attempts end"));
    }
    script.attempts = Some(attempts);
    Ok(())
}

#[given(regex = r#"^step "([^"]*)" changes its input "([^"]*)" to (.+)$"#)]
fn step_changes_its_input(
    world: &mut World,
    step_name: String,
    key: String,
    value: String,
) -> Result<(), ModelError> {
    let value = json(&value)?;
    world
        .model
        .step_mut(&step_name)?
        .actions
        .push(StepAction::ChangeInput { key, value });
    Ok(())
}

#[given(expr = "step {string} cannot be built because its constructor fails with {string}")]
fn step_cannot_be_built(
    world: &mut World,
    step_name: String,
    message: String,
) -> Result<(), ModelError> {
    world.model.step_mut(&step_name)?.construction_failure = Some(message);
    Ok(())
}

#[given(expr = "the abnormal termination of step {string} is retriable")]
fn abnormal_termination_is_retriable(
    world: &mut World,
    step_name: String,
) -> Result<(), ModelError> {
    world
        .model
        .step_mut(&step_name)?
        .abnormal_termination_retriable = true;
    Ok(())
}

#[given(expr = "step {string} allows {int} retries")]
fn step_allows_retries(
    world: &mut World,
    step_name: String,
    retries: u32,
) -> Result<(), ModelError> {
    world.model.step_mut(&step_name)?.retries = retries;
    Ok(())
}
