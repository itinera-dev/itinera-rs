//! Service providers defined before the workflow, and the workflow's bindings of them.

use cucumber::given;

use crate::model::{ModelError, Provider, Scope, ServiceType};
use crate::sentences::Names;
use crate::world::World;

#[given(expr = "the service provider {string} makes a {service} for {string}, once per journey")]
fn the_service_provider_makes_once_per_journey(
    world: &mut World,
    provider: String,
    service_type: ServiceType,
    key: String,
) {
    let provider = Provider::new(provider, key, service_type, Scope::Journey);
    world.model.define_provider(provider);
}

#[given(expr = "the service provider {string} makes a {service} for {string}, for each attempt")]
fn the_service_provider_makes_for_each_attempt(
    world: &mut World,
    provider: String,
    service_type: ServiceType,
    key: String,
) {
    let provider = Provider::new(provider, key, service_type, Scope::Attempt);
    world.model.define_provider(provider);
}

#[given(expr = "the workflow binds the service provider {string}")]
fn the_workflow_binds_the_service_provider(
    world: &mut World,
    provider: String,
) -> Result<(), ModelError> {
    world.model.bind_provider(&provider, Vec::new())
}

#[given(expr = "the workflow binds the service provider {string} to the step {string}")]
fn the_workflow_binds_the_service_provider_to_the_step(
    world: &mut World,
    provider: String,
    step_name: String,
) -> Result<(), ModelError> {
    world.model.bind_provider(&provider, vec![step_name])
}

#[given(expr = "the workflow binds the service provider {string} to the steps {names}")]
fn the_workflow_binds_the_service_provider_to_the_steps(
    world: &mut World,
    provider: String,
    steps: Names,
) -> Result<(), ModelError> {
    world.model.bind_provider(&provider, steps.into())
}
