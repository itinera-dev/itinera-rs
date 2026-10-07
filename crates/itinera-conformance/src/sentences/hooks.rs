//! Hooks, lifecycles and roles.

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

#[given(expr = "the workflow provides the role {string} with the operation {string}")]
fn the_workflow_provides_the_role(
    world: &mut World,
    role: String,
    operation: String,
) -> Result<(), ModelError> {
    world
        .model
        .workflow_mut()?
        .roles
        .entry(role)
        .or_default()
        .operations
        .insert(operation);
    Ok(())
}

#[given(
    expr = "the hook {string} of policy {string} requests the role {string} and calls its operation {string}"
)]
fn the_hook_requests_the_role(
    world: &mut World,
    hook: String,
    policy: String,
    role: String,
    operation: String,
) -> Result<(), ModelError> {
    let script = world.model.hook_mut(&policy, &hook)?;
    script.requests.push(HookRequest::Role(role.clone()));
    script
        .actions
        .push(HookAction::CallRole { role, operation });
    Ok(())
}

#[given(expr = "the operation {string} of the role {string} throws")]
fn the_operation_of_the_role_throws(
    world: &mut World,
    operation: String,
    role: String,
) -> Result<(), ModelError> {
    let role = world.model.role_mut(&role)?;
    if !role.operations.contains(&operation) {
        return Err(ModelError::UnknownOperation(operation));
    }
    role.failing.insert(operation);
    Ok(())
}
