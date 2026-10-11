//! Outcomes: what each step was built with, and how many times it was built.

use std::fmt::Arguments;
use std::num::NonZeroU32;

use cucumber::then;
use serde_json::Value;

use crate::model::{ModelError, Reason, json};
use crate::sentences::{Unmet, holds};
use crate::witness::{Build, Rescued};
use crate::world::World;

#[then(regex = r#"^step "([^"]*)" was built with input "([^"]*)" = (.+)$"#)]
fn step_was_built_with_input(
    world: &mut World,
    step_name: String,
    key: String,
    value: String,
) -> Result<(), Unmet> {
    built_with(world, &step_name, &key, Some(json(&value)?))
}

#[then(expr = "step {string} was built with input {string} absent")]
fn step_was_built_with_input_absent(
    world: &mut World,
    step_name: String,
    key: String,
) -> Result<(), Unmet> {
    built_with(world, &step_name, &key, None)
}

#[then(regex = r#"^step "([^"]*)" was built for attempt (\d+) with input "([^"]*)" = (.+)$"#)]
fn step_was_built_for_attempt_with_input(
    world: &mut World,
    step_name: String,
    attempt: NonZeroU32,
    key: String,
    value: String,
) -> Result<(), Unmet> {
    let builds: Vec<Build> = builds_of(world, &step_name)?
        .into_iter()
        .filter(|build| is_of_attempt(build, attempt))
        .collect();
    received_input(
        &builds,
        &key,
        Some(json(&value)?),
        format_args!("\"{step_name}\" built for attempt {attempt}"),
    )
}

fn is_of_attempt(build: &Build, attempt: NonZeroU32) -> bool {
    build.attempt == attempt
}

/// Holds when a build of the step received this value for the input, or received it absent.
fn built_with(world: &World, step: &str, key: &str, value: Option<Value>) -> Result<(), Unmet> {
    received_input(
        &builds_of(world, step)?,
        key,
        value,
        format_args!("\"{step}\" built"),
    )
}

/// Holds when one of these builds received this value for the input, or received it absent.
fn received_input(
    builds: &[Build],
    key: &str,
    value: Option<Value>,
    built: Arguments<'_>,
) -> Result<(), Unmet> {
    let found: Vec<Option<Value>> = builds
        .iter()
        .filter_map(|build| build.inputs.get(key))
        .cloned()
        .collect();
    holds(
        found.contains(&value),
        format_args!("{built} with \"{key}\" = {}", written(value.as_ref())),
        format_args!("the values {found:?}"),
    )
}

/// The value as the cases write it, or `absent`.
fn written(value: Option<&Value>) -> String {
    value.map_or_else(|| "absent".to_owned(), Value::to_string)
}

#[then(regex = r#"^step "([^"]*)" was built with rescued step data "([^"]*)" = (.+)$"#)]
fn step_was_built_with_rescued_step_data(
    world: &mut World,
    step_name: String,
    key: String,
    value: String,
) -> Result<(), Unmet> {
    let value = Some(json(&value)?);
    rescued(world, &step_name, Expected::Data(key, value))
}

#[then(expr = "step {string} was built with rescued step data {string} absent")]
fn step_was_built_with_rescued_step_data_absent(
    world: &mut World,
    step_name: String,
    key: String,
) -> Result<(), Unmet> {
    rescued(world, &step_name, Expected::Data(key, None))
}

#[then(
    regex = r#"^step "([^"]*)" received the failure reason with code "([^"]*)", message "([^"]*)" and details (.+)$"#
)]
fn step_received_the_failure_reason(
    world: &mut World,
    step_name: String,
    code: String,
    message: String,
    details: String,
) -> Result<(), Unmet> {
    let reason = Reason {
        code,
        message: Some(message),
        details: Some(json(&details)?),
    };
    rescued(world, &step_name, Expected::Reason(reason))
}

#[then(expr = "step {string} received the error with the message {string}")]
fn step_received_the_error_with_the_message(
    world: &mut World,
    step_name: String,
    message: String,
) -> Result<(), Unmet> {
    rescued(world, &step_name, Expected::Error(message))
}

/// What a sentence says a rescue step received from the step it rescues, on one of its builds.
#[derive(Debug)]
enum Expected {
    /// Data the failed attempt contributed under this key, `None` when absent.
    Data(String, Option<Value>),
    Reason(Reason),
    /// An error with this message.
    Error(String),
}

impl Expected {
    fn is_in(&self, got: &Rescued) -> bool {
        match self {
            Self::Data(key, value) => got.data.get(key) == Some(value),
            Self::Reason(reason) => got.reason.as_ref() == Some(reason),
            Self::Error(message) => got.error.as_ref() == Some(message),
        }
    }
}

/// Holds when a build of the rescue step received what is expected from the step it rescues.
fn rescued(world: &World, step: &str, expected: Expected) -> Result<(), Unmet> {
    let builds = builds_of(world, step)?;
    holds(
        builds
            .iter()
            .map(|build| &build.rescued)
            .any(|got| expected.is_in(got)),
        format_args!("\"{step}\" built with {expected:?}"),
        format_args!("the builds {builds:?}"),
    )
}

/// Every factory builds a new step for each attempt, which it returns by value, so counting the
/// builds counts new instances.
#[then(expr = "step {string} was built {int} times, each time as a new instance")]
fn step_was_built_times(world: &mut World, step_name: String, times: usize) -> Result<(), Unmet> {
    let found = builds_of(world, &step_name)?.len();
    holds(
        found == times,
        format_args!("\"{step_name}\" built {times} times"),
        format_args!("{found} builds"),
    )
}

fn is_of_step(build: &Build, step: &str) -> bool {
    build.step == step
}

fn builds_of(world: &World, step: &str) -> Result<Vec<Build>, ModelError> {
    Ok(world
        .witnessed()?
        .builds()
        .into_iter()
        .filter(|build| is_of_step(build, step))
        .collect())
}
