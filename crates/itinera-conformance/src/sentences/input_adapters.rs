//! Input adapters.

use cucumber::given;

use super::Names;
use crate::model::{Adapter, Answer, ModelError, ValueType, json};
use crate::world::World;

#[given(expr = "the workflow declares the input adapter {string} for the steps {names}")]
fn the_workflow_declares_the_input_adapter(
    world: &mut World,
    adapter: String,
    steps: Names,
) -> Result<(), ModelError> {
    if world.model.adapter_mut(&adapter).is_ok() {
        return Err(ModelError::AdapterDeclaredTwice(adapter));
    }
    world.model.workflow_mut()?.adapters.push(Adapter {
        name: adapter,
        steps: steps.into(),
        answers: Vec::new(),
        requests: Vec::new(),
    });
    Ok(())
}

/// Both "returns" sentences: "nothing", or a value written as JSON.
#[given(
    regex = r#"^the input adapter "([^"]*)" returns (nothing|[\[{"0-9-].*|true|false|null) for "([^"]*)"$"#
)]
fn the_input_adapter_returns(
    world: &mut World,
    adapter: String,
    answer: String,
    key: String,
) -> Result<(), ModelError> {
    let answer = match answer.as_str() {
        "nothing" => Answer::Nothing,
        value => Answer::Value(json(value)?),
    };
    answers(world, &adapter, key, answer)
}

#[given(expr = "the input adapter {string} fails with {string} for {string}")]
fn the_input_adapter_fails(
    world: &mut World,
    adapter: String,
    message: String,
    key: String,
) -> Result<(), ModelError> {
    answers(world, &adapter, key, Answer::Fails(message))
}

fn answers(
    world: &mut World,
    adapter: &str,
    key: String,
    answer: Answer,
) -> Result<(), ModelError> {
    world
        .model
        .adapter_mut(adapter)?
        .answers
        .push((key, answer));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_input_adapter_with_a_used_name_is_a_case_error() {
        let mut world = World::default();
        world
            .model
            .declare("orders".to_owned(), vec!["charge".to_owned()])
            .unwrap();
        let steps = || "\"charge\"".parse::<Names>().unwrap();
        the_workflow_declares_the_input_adapter(&mut world, "pricing".to_owned(), steps()).unwrap();

        let declared =
            the_workflow_declares_the_input_adapter(&mut world, "pricing".to_owned(), steps());

        assert_eq!(
            declared,
            Err(ModelError::AdapterDeclaredTwice("pricing".to_owned()))
        );
    }
}
