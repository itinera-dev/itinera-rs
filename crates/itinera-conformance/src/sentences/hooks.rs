//! Hooks, lifecycles and roles: what hooks do, and what happened to them.

mod happened;
mod roles;

use cucumber::given;

use super::policies::requests;
use crate::model::{HookAction, HookRequest, HookReturn, ModelError, ValueType, json};
use crate::world::World;

#[given(expr = "the hook {string} of policy {string} returns FinishWorkflow")]
fn the_hook_returns_finish_workflow(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    returns(world, &policy, &hook, HookReturn::FinishWorkflow)
}

#[given(expr = "the hook {string} of policy {string} returns FailWorkflow with code {string}")]
fn the_hook_returns_fail_workflow(
    world: &mut World,
    hook: String,
    policy: String,
    code: String,
) -> Result<(), ModelError> {
    returns(world, &policy, &hook, HookReturn::FailWorkflow { code })
}

#[given(expr = "the hook {string} of policy {string} throws")]
fn the_hook_throws(world: &mut World, hook: String, policy: String) -> Result<(), ModelError> {
    returns(world, &policy, &hook, HookReturn::Fails(None))
}

#[given(expr = "the hook {string} of policy {string} fails with {string}")]
fn the_hook_fails_with(
    world: &mut World,
    hook: String,
    policy: String,
    message: String,
) -> Result<(), ModelError> {
    returns(world, &policy, &hook, HookReturn::Fails(Some(message)))
}

fn returns(
    world: &mut World,
    policy: &str,
    hook: &str,
    returned: HookReturn,
) -> Result<(), ModelError> {
    let script = world.model.hook_mut(policy, hook)?;
    if script.returns != HookReturn::Nothing {
        return Err(ModelError::StatedTwice("what the hook returns"));
    }
    script.returns = returned;
    Ok(())
}

#[given(
    expr = "the hook {string} of policy {string} requests data from the workflow {string} of type {type}"
)]
fn the_hook_requests_data_from_the_workflow(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    let request = HookRequest::DataFromWorkflow {
        key,
        value_type,
        optional: false,
    };
    requests(world, &policy, &hook, request)
}

#[given(
    expr = "the hook {string} of policy {string} requests optional data from the workflow {string} of type {type}"
)]
fn the_hook_requests_optional_data_from_the_workflow(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    let request = HookRequest::DataFromWorkflow {
        key,
        value_type,
        optional: true,
    };
    requests(world, &policy, &hook, request)
}

#[given(expr = "the hook {string} of policy {string} requests the attempt number")]
fn the_hook_requests_the_attempt_number(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    requests(world, &policy, &hook, HookRequest::AttemptNumber)
}

#[given(expr = "the hook {string} of policy {string} requests the journey ID")]
fn the_hook_requests_the_journey_id(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    requests(world, &policy, &hook, HookRequest::JourneyId)
}

#[given(regex = r#"^the hook "([^"]*)" of policy "([^"]*)" contributes "([^"]*)" = (.+)$"#)]
fn the_hook_contributes(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
    value: String,
) -> Result<(), ModelError> {
    let action = HookAction::Contribute {
        key,
        value: json(&value)?,
    };
    world.model.hook_mut(&policy, &hook)?.actions.push(action);
    Ok(())
}

#[given(
    expr = "the hook {string} of policy {string} counts its calls and contributes the count as {string}"
)]
fn the_hook_counts_its_calls(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
) -> Result<(), ModelError> {
    let action = HookAction::ContributeCallCount { key };
    world.model.hook_mut(&policy, &hook)?.actions.push(action);
    Ok(())
}

#[given(expr = "the policy {string} fails when it is built")]
fn the_policy_fails_when_built(world: &mut World, policy: String) -> Result<(), ModelError> {
    fails_when_built(world, &policy, None)
}

#[given(expr = "the policy {string} fails with {string} when it is built")]
fn the_policy_fails_with_when_built(
    world: &mut World,
    policy: String,
    message: String,
) -> Result<(), ModelError> {
    fails_when_built(world, &policy, Some(message))
}

fn fails_when_built(
    world: &mut World,
    policy: &str,
    message: Option<String>,
) -> Result<(), ModelError> {
    let policy = world.model.policy_mut(policy)?;
    if policy.construction_failure.is_some() {
        return Err(ModelError::StatedTwice("how the policy fails when built"));
    }
    policy.construction_failure = Some(message);
    Ok(())
}
