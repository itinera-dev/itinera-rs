//! Policies, and what their hooks request from the step.

use cucumber::given;

use super::Names;
use crate::model::{HookAction, HookRequest, ModelError, ValueType, json};
use crate::world::World;

#[given(expr = "a step policy {string} defines the hook {string}")]
fn a_step_policy_defines_the_hook(
    world: &mut World,
    policy: String,
    hook: String,
) -> Result<(), ModelError> {
    world.model.define(policy, &hook, true)
}

#[given(expr = "a workflow policy {string} defines the hook {string}")]
fn a_workflow_policy_defines_the_hook(
    world: &mut World,
    policy: String,
    hook: String,
) -> Result<(), ModelError> {
    world.model.define(policy, &hook, false)
}

#[given(expr = "step {string} has the policies {names}")]
fn step_has_the_policies(
    world: &mut World,
    step_name: String,
    policies: Names,
) -> Result<(), ModelError> {
    policies
        .0
        .iter()
        .try_for_each(|policy| world.model.require_step_policy(policy))?;
    world.model.step_mut(&step_name)?;
    world
        .model
        .workflow_mut()?
        .step_policies
        .entry(step_name)
        .or_default()
        .extend(policies.0);
    Ok(())
}

#[given(expr = "the workflow has the policies {names}")]
fn the_workflow_has_the_policies(world: &mut World, policies: Names) -> Result<(), ModelError> {
    policies
        .0
        .iter()
        .try_for_each(|policy| world.model.require_workflow_policy(policy))?;
    world.model.workflow_mut()?.policies.extend(policies.0);
    Ok(())
}

#[given(expr = "the hook {string} of policy {string} requests step data {string} of type {type}")]
fn the_hook_requests_step_data(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    let request = HookRequest::StepData {
        key,
        value_type,
        optional: false,
    };
    requests(world, &policy, &hook, request)
}

#[given(
    expr = "the hook {string} of policy {string} requests optional step data {string} of type {type}"
)]
fn the_hook_requests_optional_step_data(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
    value_type: ValueType,
) -> Result<(), ModelError> {
    let request = HookRequest::StepData {
        key,
        value_type,
        optional: true,
    };
    requests(world, &policy, &hook, request)
}

#[given(
    regex = r#"^the hook "([^"]*)" of policy "([^"]*)" changes the step data "([^"]*)" it received to (.+)$"#
)]
fn the_hook_changes_the_step_data_it_received(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
    value: String,
) -> Result<(), ModelError> {
    let action = HookAction::ChangeStepData {
        key,
        value: json(&value)?,
    };
    world.model.hook_mut(&policy, &hook)?.actions.push(action);
    Ok(())
}

#[given(expr = "the hook {string} of policy {string} requests the step name")]
fn the_hook_requests_the_step_name(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    requests(world, &policy, &hook, HookRequest::StepName)
}

#[given(expr = "the hook {string} of policy {string} requests the failure reason")]
fn the_hook_requests_the_failure_reason(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    requests(
        world,
        &policy,
        &hook,
        HookRequest::FailureReason { optional: false },
    )
}

#[given(expr = "the hook {string} of policy {string} requests the optional failure reason")]
fn the_hook_requests_the_optional_failure_reason(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    requests(
        world,
        &policy,
        &hook,
        HookRequest::FailureReason { optional: true },
    )
}

#[given(expr = "the hook {string} of policy {string} requests the failure cause")]
fn the_hook_requests_the_failure_cause(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    requests(world, &policy, &hook, HookRequest::FailureCause)
}

#[given(expr = "the hook {string} of policy {string} requests the retry cause")]
fn the_hook_requests_the_retry_cause(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    requests(world, &policy, &hook, HookRequest::RetryCause)
}

#[given(expr = "the hook {string} of policy {string} requests the error")]
fn the_hook_requests_the_error(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    requests(world, &policy, &hook, HookRequest::Error)
}

pub(super) fn requests(
    world: &mut World,
    policy: &str,
    hook: &str,
    request: HookRequest,
) -> Result<(), ModelError> {
    world.model.hook_mut(policy, hook)?.requests.push(request);
    Ok(())
}
