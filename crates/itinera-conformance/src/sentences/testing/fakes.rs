//! Fakes that replace service providers, and the calls each fake had.

use cucumber::{given, then};

use crate::model::{Fake, Faking, Implementation, ModelError, json};
use crate::sentences::{Unmet, holds};
use crate::world::World;

/// The fakes that make labellers, with no data or with data written as JSON.
#[given(
    regex = r#"^the test replaces the service provider "([^"]*)" with a fake that makes instances of (first labeller|second labeller)(?: and supplies the data (.+))?$"#
)]
fn the_test_replaces_the_provider_with_a_fake_that_makes_instances(
    world: &mut World,
    provider: String,
    implementation: Implementation,
    data: String,
) -> Result<(), ModelError> {
    let makes = Faking::Labellers {
        implementation,
        data: (!data.is_empty()).then(|| json(&data)).transpose()?,
    };
    replaces(world, provider, makes);
    Ok(())
}

#[given(
    expr = "the test replaces the service provider {string} with a fake that fails with {string}"
)]
fn the_test_replaces_the_provider_with_a_fake_that_fails(
    world: &mut World,
    provider: String,
    message: String,
) {
    replaces(world, provider, Faking::Fails(message));
}

#[given(expr = "the test replaces the service provider {string} with a fake that makes meters")]
fn the_test_replaces_the_provider_with_a_fake_that_makes_meters(
    world: &mut World,
    provider: String,
) {
    replaces(world, provider, Faking::Meters);
}

fn replaces(world: &mut World, provider: String, makes: Faking) {
    world.model.testing.fakes.push(Fake { provider, makes });
}

#[then(expr = "the fake of {string} was called {int} times")]
fn the_fake_was_called(world: &mut World, provider: String, times: u32) -> Result<(), Unmet> {
    let found = world
        .tested
        .fake_calls
        .get(&provider)
        .ok_or_else(|| no_fake(&provider))?
        .count();
    holds(
        found == times,
        format_args!("the fake of \"{provider}\" called {times} times"),
        format_args!("{found} calls"),
    )
}

fn no_fake(provider: &str) -> Unmet {
    Unmet::Case(format!(
        "no fake replaces the service provider \"{provider}\""
    ))
}
