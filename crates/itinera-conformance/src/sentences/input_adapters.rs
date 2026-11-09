//! Input adapters.

use cucumber::given;

use super::Names;
use crate::model::{Answer, Answering, ModelError, ValueType, json};
use crate::world::World;

#[given(expr = "the workflow declares the input adapter {string} for the steps {names}")]
fn the_workflow_declares_the_input_adapter(
    world: &mut World,
    adapter: String,
    steps: Names,
) -> Result<(), ModelError> {
    world.model.bind_adapter(&adapter, steps.into())
}

#[given(expr = "the workflow binds the input adapter {string} to the step {string}")]
fn the_workflow_binds_the_input_adapter_to_the_step(
    world: &mut World,
    adapter: String,
    step_name: String,
) -> Result<(), ModelError> {
    world.model.bind_adapter(&adapter, vec![step_name])
}

#[given(expr = "the workflow binds the input adapter {string} to the steps {names}")]
fn the_workflow_binds_the_input_adapter_to_the_steps(
    world: &mut World,
    adapter: String,
    steps: Names,
) -> Result<(), ModelError> {
    world.model.bind_adapter(&adapter, steps.into())
}

/// Both "returns" sentences: "nothing", or a value written as JSON. A string is matched up to
/// its closing quote, so that a read from the data bag, which names a key, is not one.
#[given(
    regex = r#"^the input adapter "([^"]*)" returns (nothing|[\[{0-9-].*|"(?:[^"\\]|\\.)*"|true|false|null) for "([^"]*)"$"#
)]
fn the_input_adapter_returns(
    world: &mut World,
    adapter: String,
    answer: String,
    key: String,
) -> Result<(), ModelError> {
    answers(world, &adapter, key, None, returned(&answer)?)
}

/// Both "returns ... when step ... requests" sentences: "nothing", or a value written as JSON.
#[given(
    regex = r#"^the input adapter "([^"]*)" returns (nothing|[\[{0-9-].*|"(?:[^"\\]|\\.)*"|true|false|null) when step "([^"]*)" requests "([^"]*)"$"#
)]
fn the_input_adapter_returns_when_step_requests(
    world: &mut World,
    adapter: String,
    answer: String,
    step_name: String,
    key: String,
) -> Result<(), ModelError> {
    answers(world, &adapter, key, Some(step_name), returned(&answer)?)
}

fn returned(answer: &str) -> Result<Answer, ModelError> {
    Ok(match answer {
        "nothing" => Answer::Nothing,
        value => Answer::Value(json(value)?),
    })
}

#[given(expr = "the input adapter {string} fails with {string} for {string}")]
fn the_input_adapter_fails(
    world: &mut World,
    adapter: String,
    message: String,
    key: String,
) -> Result<(), ModelError> {
    answers(world, &adapter, key, None, Answer::Fails(message))
}

#[given(
    expr = "the input adapter {string} fails with {string} when step {string} requests {string}"
)]
fn the_input_adapter_fails_when_step_requests(
    world: &mut World,
    adapter: String,
    message: String,
    step_name: String,
    key: String,
) -> Result<(), ModelError> {
    answers(
        world,
        &adapter,
        key,
        Some(step_name),
        Answer::Fails(message),
    )
}

#[given(
    expr = "the input adapter {string} returns {string} of type {type}, read from the data bag, for {string}"
)]
fn the_input_adapter_returns_read_from_the_data_bag(
    world: &mut World,
    adapter: String,
    read: String,
    value_type: ValueType,
    key: String,
) -> Result<(), ModelError> {
    answers(
        world,
        &adapter,
        key,
        None,
        Answer::ReadFromDataBag(read, value_type),
    )
}

#[given(expr = "the input adapter {string} requests the step name and returns it for {string}")]
fn the_input_adapter_requests_the_step_name_and_returns_it(
    world: &mut World,
    adapter: String,
    key: String,
) -> Result<(), ModelError> {
    answers(world, &adapter, key, None, Answer::StepName)
}

fn answers(
    world: &mut World,
    adapter: &str,
    key: String,
    step: Option<String>,
    answer: Answer,
) -> Result<(), ModelError> {
    world
        .model
        .adapter_mut(adapter)?
        .answers
        .push(Answering { key, step, answer });
    Ok(())
}

#[given(
    expr = "the input adapter {string} requests data from the workflow {string} of type {type}"
)]
fn the_input_adapter_requests_data_from_the_workflow(
    world: &mut World,
    adapter: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    world
        .model
        .adapter_mut(&adapter)?
        .requests
        .push((key, value_type));
    Ok(())
}

#[given(expr = "the input adapter {string} requests the data bag")]
fn the_input_adapter_requests_the_data_bag(
    world: &mut World,
    adapter: String,
) -> Result<(), ModelError> {
    world.model.adapter_mut(&adapter)?.data_bag = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_input_adapter_with_a_used_name_declares_another_adapter() {
        let mut world = World::default();
        world
            .model
            .declare("orders".to_owned(), vec!["charge".to_owned()])
            .unwrap();
        let steps = || Names::from(vec!["charge".to_owned()]);
        the_workflow_declares_the_input_adapter(&mut world, "pricing".to_owned(), steps()).unwrap();

        the_workflow_declares_the_input_adapter(&mut world, "pricing".to_owned(), steps()).unwrap();

        assert_eq!(world.model.workflow().unwrap().adapters.len(), 2);
    }
}
