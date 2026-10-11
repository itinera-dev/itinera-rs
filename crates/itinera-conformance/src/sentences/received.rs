//! Outcomes: what each step was built with, and what each hook received, as the witness of the
//! scenario recorded them.

use std::fmt::Arguments;
use std::num::NonZeroU32;

use cucumber::then;
use serde_json::Value;

use super::{Unmet, holds};
use crate::model::{Hook, ModelError, Reason, json};
use crate::witness::{Build, Call, Received};
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
    let builds: Vec<Build> = builds_of(world, &step_name)
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
        &builds_of(world, step),
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

/// Every factory builds a new step for each attempt, which it returns by value, so counting the
/// builds counts new instances.
#[then(expr = "step {string} was built {int} times, each time as a new instance")]
fn step_was_built_times(world: &mut World, step_name: String, times: usize) -> Result<(), Unmet> {
    let found = builds_of(world, &step_name).len();
    holds(
        found == times,
        format_args!("\"{step_name}\" built {times} times"),
        format_args!("{found} builds"),
    )
}

fn is_of_step(build: &Build, step: &str) -> bool {
    build.step == step
}

fn builds_of(world: &World, step: &str) -> Vec<Build> {
    world
        .witness
        .builds()
        .into_iter()
        .filter(|build| is_of_step(build, step))
        .collect()
}

#[then(regex = r#"^the hook "([^"]*)" of policy "([^"]*)" received step data "([^"]*)" = (.+)$"#)]
fn the_hook_received_step_data(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
    value: String,
) -> Result<(), Unmet> {
    let value = Some(json(&value)?);
    received(world, &policy, &hook, Expected::StepData(key, value))
}

#[then(expr = "the hook {string} of policy {string} received step data {string} absent")]
fn the_hook_received_step_data_absent(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
) -> Result<(), Unmet> {
    received(world, &policy, &hook, Expected::StepData(key, None))
}

#[then(
    regex = r#"^the hook "([^"]*)" of policy "([^"]*)" received data from the workflow "([^"]*)" = (.+)$"#
)]
fn the_hook_received_data_from_the_workflow(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
    value: String,
) -> Result<(), Unmet> {
    let value = Some(json(&value)?);
    received(world, &policy, &hook, Expected::WorkflowData(key, value))
}

#[then(
    expr = "the hook {string} of policy {string} received data from the workflow {string} absent"
)]
fn the_hook_received_data_from_the_workflow_absent(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
) -> Result<(), Unmet> {
    received(world, &policy, &hook, Expected::WorkflowData(key, None))
}

#[then(expr = "the hook {string} of policy {string} received the step name {string}")]
fn the_hook_received_the_step_name(
    world: &mut World,
    hook: String,
    policy: String,
    step_name: String,
) -> Result<(), Unmet> {
    received(world, &policy, &hook, Expected::StepName(step_name))
}

#[then(expr = "the hook {string} of policy {string} received the attempt number {int}")]
fn the_hook_received_the_attempt_number(
    world: &mut World,
    hook: String,
    policy: String,
    attempt: NonZeroU32,
) -> Result<(), Unmet> {
    received(world, &policy, &hook, Expected::Attempt(attempt))
}

#[then(expr = "the hook {string} of policy {string} received the journey ID {string}")]
fn the_hook_received_the_journey_id(
    world: &mut World,
    hook: String,
    policy: String,
    id: String,
) -> Result<(), Unmet> {
    received(world, &policy, &hook, Expected::JourneyId(id))
}

#[then(expr = "the hook {string} of policy {string} received the cause {string}")]
fn the_hook_received_the_cause(
    world: &mut World,
    hook: String,
    policy: String,
    cause: String,
) -> Result<(), Unmet> {
    received(world, &policy, &hook, Expected::Cause(cause))
}

#[then(
    regex = r#"^the hook "([^"]*)" of policy "([^"]*)" received the failure reason with code "([^"]*)", message "([^"]*)" and details (.+)$"#
)]
fn the_hook_received_the_failure_reason(
    world: &mut World,
    hook: String,
    policy: String,
    code: String,
    message: String,
    details: String,
) -> Result<(), Unmet> {
    let reason = Some(Reason {
        code,
        message: Some(message),
        details: Some(json(&details)?),
    });
    received(world, &policy, &hook, Expected::Reason(reason))
}

#[then(expr = "the hook {string} of policy {string} received no failure reason")]
fn the_hook_received_no_failure_reason(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), Unmet> {
    received(world, &policy, &hook, Expected::Reason(None))
}

#[then(expr = "the hook {string} of policy {string} received an error")]
fn the_hook_received_an_error(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), Unmet> {
    received(world, &policy, &hook, Expected::AnError)
}

#[then(expr = "the hook {string} of policy {string} received the error with the message {string}")]
fn the_hook_received_the_error_with_the_message(
    world: &mut World,
    hook: String,
    policy: String,
    message: String,
) -> Result<(), Unmet> {
    received(world, &policy, &hook, Expected::Error(message))
}

/// What a sentence says a hook received, on at least one of its calls.
#[derive(Debug)]
enum Expected {
    /// Data from the step under this key, `None` when absent.
    StepData(String, Option<Value>),
    /// Data from the workflow under this key, `None` when absent.
    WorkflowData(String, Option<Value>),
    StepName(String),
    Attempt(NonZeroU32),
    JourneyId(String),
    Cause(String),
    /// The failure reason, `None` when it was absent.
    Reason(Option<Reason>),
    AnError,
    /// An error with this message.
    Error(String),
}

impl Expected {
    fn is_in(&self, got: &Received) -> bool {
        match self {
            Self::StepData(key, value) => got.step_data.get(key) == Some(value),
            Self::WorkflowData(key, value) => got.workflow_data.get(key) == Some(value),
            Self::StepName(step) => got.step_name.as_ref() == Some(step),
            Self::Attempt(attempt) => got.attempt == Some(*attempt),
            Self::JourneyId(id) => got.journey_id.as_ref() == Some(id),
            Self::Cause(cause) => got.cause.as_ref() == Some(cause),
            Self::Reason(reason) => got.reason.as_ref() == Some(reason),
            Self::AnError => got.error.is_some(),
            Self::Error(message) => got.error.as_ref() == Some(message),
        }
    }
}

/// Holds when a call of the policy's hook received what is expected.
fn received(world: &World, policy: &str, hook: &str, expected: Expected) -> Result<(), Unmet> {
    let calls = calls_of(world, policy, hook)?;
    holds(
        calls
            .iter()
            .map(|call| &call.received)
            .any(|got| expected.is_in(got)),
        format_args!("the hook \"{hook}\" of policy \"{policy}\" to receive {expected:?}"),
        format_args!("the calls {calls:?}"),
    )
}

/// The calls of the policy's hook, in the order they happened.
pub(super) fn calls_of(world: &World, policy: &str, hook: &str) -> Result<Vec<Call>, ModelError> {
    let hook = Hook::named(hook)?;
    Ok(world
        .witness
        .calls()
        .into_iter()
        .filter(|call| is_of_hook(call, policy, hook))
        .collect())
}

fn is_of_hook(call: &Call, policy: &str, hook: Hook) -> bool {
    call.policy == policy && call.hook == hook
}
