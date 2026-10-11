//! Outcomes: what each hook received.

use std::num::NonZeroU32;

use cucumber::then;
use serde_json::Value;

use super::calls_of;
use crate::model::{Reason, json};
use crate::sentences::{Unmet, holds};
use crate::witness::Received;
use crate::world::World;

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
