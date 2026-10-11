//! Declaring rescue steps, and what they request from the steps they rescue.

use cucumber::given;

use crate::model::{Input, ModelError, RescueRequest, ValueType};
use crate::world::World;

#[given(expr = "step {string} has a rescue step")]
fn step_has_a_rescue_step(world: &mut World, step_name: String) -> Result<(), ModelError> {
    world.model.rescue(&step_name)
}

#[given(expr = "step {string} requests rescued step data {string} of type {type}")]
fn step_requests_rescued_step_data(
    world: &mut World,
    step_name: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    let input = Input {
        key,
        value_type,
        optional: false,
    };
    request(world, &step_name, RescueRequest::Data(input))
}

#[given(expr = "step {string} requests optional rescued step data {string} of type {type}")]
fn step_requests_optional_rescued_step_data(
    world: &mut World,
    step_name: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    let input = Input {
        key,
        value_type,
        optional: true,
    };
    request(world, &step_name, RescueRequest::Data(input))
}

#[given(expr = "step {string} requests the failure reason")]
fn step_requests_the_failure_reason(
    world: &mut World,
    step_name: String,
) -> Result<(), ModelError> {
    request(world, &step_name, RescueRequest::Reason)
}

#[given(expr = "step {string} requests the error")]
fn step_requests_the_error(world: &mut World, step_name: String) -> Result<(), ModelError> {
    request(world, &step_name, RescueRequest::Error)
}

fn request(world: &mut World, step: &str, request: RescueRequest) -> Result<(), ModelError> {
    world
        .model
        .rescue_step_mut(step)?
        .rescue_requests
        .push(request);
    Ok(())
}
