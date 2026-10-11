//! Services: what the workflow's service providers request.

use cucumber::given;

use crate::model::{HookRequest, ModelError, ValueType};
use crate::world::World;

#[given(expr = "the service provider {string} requests the step name")]
fn the_service_provider_requests_the_step_name(
    world: &mut World,
    provider: String,
) -> Result<(), ModelError> {
    request(world, &provider, HookRequest::StepName)
}

#[given(expr = "the service provider {string} requests the attempt number")]
fn the_service_provider_requests_the_attempt_number(
    world: &mut World,
    provider: String,
) -> Result<(), ModelError> {
    request(world, &provider, HookRequest::AttemptNumber)
}

#[given(expr = "the service provider {string} requests the journey ID")]
fn the_service_provider_requests_the_journey_id(
    world: &mut World,
    provider: String,
) -> Result<(), ModelError> {
    request(world, &provider, HookRequest::JourneyId)
}

#[given(
    expr = "the service provider {string} requests data from the workflow {string} of type {type}"
)]
fn the_service_provider_requests_data_from_the_workflow(
    world: &mut World,
    provider: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    let data = HookRequest::DataFromWorkflow {
        key,
        value_type,
        optional: false,
    };
    request(world, &provider, data)
}

#[given(
    expr = "the service provider {string} requests optional data from the workflow {string} of type {type}"
)]
fn the_service_provider_requests_optional_data_from_the_workflow(
    world: &mut World,
    provider: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    let data = HookRequest::DataFromWorkflow {
        key,
        value_type,
        optional: true,
    };
    request(world, &provider, data)
}

fn request(world: &mut World, provider: &str, request: HookRequest) -> Result<(), ModelError> {
    world.model.provider_mut(provider)?.requests.push(request);
    Ok(())
}
