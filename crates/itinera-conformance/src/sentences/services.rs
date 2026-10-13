//! Services: what steps request, and what the workflow's service providers do.

mod requests;

use cucumber::given;

use super::Names;
use crate::model::{
    Implementation, Level, ModelError, Provider, ProviderAction, ProviderFailure, Scope,
    ServiceType, json,
};
use crate::world::World;

#[given(expr = "step {string} requests the service {string} of type {service}")]
fn step_requests_the_service(
    world: &mut World,
    step_name: String,
    key: String,
    service_type: ServiceType,
) -> Result<(), ModelError> {
    world
        .model
        .step_mut(&step_name)?
        .services
        .push((key, service_type));
    Ok(())
}

#[given(
    expr = "the workflow declares the service provider {string} for {string} of type {service}, once per journey"
)]
fn the_workflow_declares_the_service_provider_once_per_journey(
    world: &mut World,
    provider: String,
    key: String,
    service_type: ServiceType,
) -> Result<(), ModelError> {
    declare(
        world,
        Provider::new(provider, key, service_type, Scope::Journey),
    )
}

#[given(
    expr = "the workflow declares the service provider {string} for {string} of type {service}, for each attempt"
)]
fn the_workflow_declares_the_service_provider_for_each_attempt(
    world: &mut World,
    provider: String,
    key: String,
    service_type: ServiceType,
) -> Result<(), ModelError> {
    declare(
        world,
        Provider::new(provider, key, service_type, Scope::Attempt),
    )
}

fn declare(world: &mut World, provider: Provider) -> Result<(), ModelError> {
    world.model.workflow_mut()?.providers.push(provider);
    Ok(())
}

#[given(expr = "the service provider {string} serves only the steps {names}")]
fn the_service_provider_serves_only_the_steps(
    world: &mut World,
    provider: String,
    steps: Names,
) -> Result<(), ModelError> {
    world.model.provider_mut(&provider)?.steps.extend(steps);
    Ok(())
}

#[given(
    expr = "the service provider {string} labels each instance with the value it received for {string}"
)]
fn the_service_provider_labels_each_instance(
    world: &mut World,
    provider: String,
    key: String,
) -> Result<(), ModelError> {
    world.model.provider_mut(&provider)?.labelled_by = Some(key);
    Ok(())
}

#[given(expr = "the service provider {string} makes instances of {implementation}")]
fn the_service_provider_makes_instances_of(
    world: &mut World,
    provider: String,
    implementation: Implementation,
) -> Result<(), ModelError> {
    let provider = world.model.provider_mut(&provider)?;
    if provider.service_type != ServiceType::Labeller {
        return Err(ModelError::NoImplementations(provider.name.clone()));
    }
    provider.implementation = implementation;
    Ok(())
}

#[given(
    regex = r#"^the service provider "([^"]*)" supplies the data ([\[{"0-9-].*|true|false|null) with each instance$"#
)]
fn the_service_provider_supplies_the_data(
    world: &mut World,
    provider: String,
    data: String,
) -> Result<(), ModelError> {
    world.model.provider_mut(&provider)?.data = Some(json(&data)?);
    Ok(())
}

#[given(expr = "the service provider {string} fails with {string}")]
fn the_service_provider_fails(
    world: &mut World,
    provider: String,
    message: String,
) -> Result<(), ModelError> {
    let failure = ProviderFailure {
        message,
        when: None,
    };
    fails(world, &provider, failure)
}

#[given(
    expr = "the service provider {string} fails with {string} when it receives {string} for {string}"
)]
fn the_service_provider_fails_when_it_receives(
    world: &mut World,
    provider: String,
    message: String,
    value: String,
    key: String,
) -> Result<(), ModelError> {
    let failure = ProviderFailure {
        message,
        when: Some((key, value)),
    };
    fails(world, &provider, failure)
}

fn fails(world: &mut World, provider: &str, failure: ProviderFailure) -> Result<(), ModelError> {
    let provider = world.model.provider_mut(provider)?;
    if provider.failure.is_some() {
        return Err(ModelError::StatedTwice("how the service provider fails"));
    }
    provider.failure = Some(failure);
    Ok(())
}

#[given(regex = r#"^the service provider "([^"]*)" emits (\w+) "([^"]*)"$"#)]
fn the_service_provider_emits(
    world: &mut World,
    provider: String,
    kind: String,
    message: String,
) -> Result<(), ModelError> {
    let level = Level::of("journey", &kind)?;
    world
        .model
        .provider_mut(&provider)?
        .actions
        .push(ProviderAction::Emit(level, message));
    Ok(())
}

#[given(expr = "the services the provider {string} makes fail with {string} when used")]
fn the_services_the_provider_makes_fail_when_used(
    world: &mut World,
    provider: String,
    message: String,
) -> Result<(), ModelError> {
    world.model.provider_mut(&provider)?.fails_when_used = Some(message);
    Ok(())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::a_labeller(ServiceType::Labeller, Ok(()))]
    #[case::a_meter(
        ServiceType::Meter,
        Err(ModelError::NoImplementations("counting".to_owned()))
    )]
    fn only_a_provider_of_a_labeller_makes_instances_of_an_implementation(
        #[case] service_type: ServiceType,
        #[case] chosen: Result<(), ModelError>,
    ) {
        let mut world = World::default();
        world
            .model
            .declare("orders".to_owned(), vec!["charge".to_owned()])
            .unwrap();
        the_workflow_declares_the_service_provider_once_per_journey(
            &mut world,
            "counting".to_owned(),
            "usage".to_owned(),
            service_type,
        )
        .unwrap();

        let made = the_service_provider_makes_instances_of(
            &mut world,
            "counting".to_owned(),
            Implementation::SecondLabeller,
        );

        assert_eq!(made, chosen);
    }
}
