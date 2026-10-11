//! Outcomes: the services each step was given, and what each service provider received.

use std::num::NonZeroU32;

use cucumber::then;

use super::Expected;
use crate::model::ModelError;
use crate::sentences::{Unmet, holds};
use crate::witness::{Given, Provision};
use crate::world::World;

#[then(expr = "the service provider {string} was called {int} times")]
fn the_service_provider_was_called(
    world: &mut World,
    provider: String,
    times: usize,
) -> Result<(), Unmet> {
    let found = provisions_of(world, &provider)?.len();
    holds(
        found == times,
        format_args!("the service provider \"{provider}\" called {times} times"),
        format_args!("{found} calls"),
    )
}

#[then(expr = "step {string} was given the service {string} labelled {string} for attempt {int}")]
fn step_was_given_the_service(
    world: &mut World,
    step_name: String,
    key: String,
    label: String,
    attempt: NonZeroU32,
) -> Result<(), Unmet> {
    let expected = Given {
        step: step_name,
        attempt,
        key,
        label,
    };
    let found = world.witnessed()?.services();
    holds(
        found.contains(&expected),
        format_args!("{expected:?}"),
        format_args!("the services given {found:?}"),
    )
}

#[then(expr = "the service provider {string} received the step name {string}")]
fn the_service_provider_received_the_step_name(
    world: &mut World,
    provider: String,
    step_name: String,
) -> Result<(), Unmet> {
    received(world, &provider, Expected::StepName(step_name))
}

#[then(expr = "the service provider {string} received the attempt number {int}")]
fn the_service_provider_received_the_attempt_number(
    world: &mut World,
    provider: String,
    attempt: NonZeroU32,
) -> Result<(), Unmet> {
    received(world, &provider, Expected::Attempt(attempt))
}

#[then(expr = "the service provider {string} received the journey ID {string}")]
fn the_service_provider_received_the_journey_id(
    world: &mut World,
    provider: String,
    id: String,
) -> Result<(), Unmet> {
    received(world, &provider, Expected::JourneyId(id))
}

#[then(expr = "the service provider {string} received data from the workflow {string} absent")]
fn the_service_provider_received_data_from_the_workflow_absent(
    world: &mut World,
    provider: String,
    key: String,
) -> Result<(), Unmet> {
    received(world, &provider, Expected::WorkflowData(key, None))
}

/// Holds when a call of the provider received what is expected.
fn received(world: &World, provider: &str, expected: Expected) -> Result<(), Unmet> {
    let provisions = provisions_of(world, provider)?;
    holds(
        provisions
            .iter()
            .map(|provision| &provision.received)
            .any(|got| expected.is_in(got)),
        format_args!("the service provider \"{provider}\" to receive {expected:?}"),
        format_args!("the calls {provisions:?}"),
    )
}

/// The calls of the provider, in the order they were made.
fn provisions_of(world: &World, provider: &str) -> Result<Vec<Provision>, ModelError> {
    Ok(world
        .witnessed()?
        .provisions()
        .into_iter()
        .filter(|provision| is_of_provider(provision, provider))
        .collect())
}

fn is_of_provider(provision: &Provision, provider: &str) -> bool {
    provision.provider == provider
}
