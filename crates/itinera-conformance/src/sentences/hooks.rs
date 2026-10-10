//! Hooks, lifecycles and roles: what hooks do, and what happened to them.

use cucumber::gherkin::Step as Sentence;
use cucumber::{given, then};
use itinera::journey::{Failure, JourneyStatus};

use super::outcomes::fails_at;
use super::policies::requests;
use super::received::calls_of;
use super::{Unmet, expect, holds, result, stream};
use crate::model::{
    Hook, HookAction, HookRequest, HookReturn, ModelError, Row, ValueType, json, rows,
};
use crate::trace::Line;
use crate::witness::{Call, Operation};
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

#[then(expr = "the hook {string} of policy {string} was called {int} times")]
fn the_hook_was_called_times(
    world: &mut World,
    hook: String,
    policy: String,
    times: usize,
) -> Result<(), Unmet> {
    called(world, &policy, &hook, times)
}

#[then(expr = "the hook {string} of policy {string} was not called")]
fn the_hook_was_not_called(world: &mut World, hook: String, policy: String) -> Result<(), Unmet> {
    called(world, &policy, &hook, 0)
}

fn called(world: &World, policy: &str, hook: &str, times: usize) -> Result<(), Unmet> {
    let found = calls_of(world, policy, hook)?.len();
    holds(
        found == times,
        format_args!("the hook \"{hook}\" of policy \"{policy}\" called {times} times"),
        format_args!("{found} calls"),
    )
}

#[then(expr = "the hooks were called in this order:")]
fn the_hooks_were_called_in_this_order(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let expected = rows(sentence)?
        .iter()
        .map(policy_and_hook)
        .collect::<Result<Vec<_>, _>>()?;
    let found: Vec<(String, Hook)> = world.witness.calls().into_iter().map(named).collect();
    holds(
        found == expected,
        format_args!("the calls {expected:?}"),
        format_args!("{found:?}"),
    )
}

fn policy_and_hook(row: &Row) -> Result<(String, Hook), ModelError> {
    Ok((
        row.required("policy")?.to_owned(),
        Hook::named(row.required("hook")?)?,
    ))
}

fn named(call: Call) -> (String, Hook) {
    (call.policy, call.hook)
}

#[then(expr = "the operation {string} of the role {string} was called {int} times")]
fn the_operation_of_the_role_was_called(
    world: &mut World,
    operation: String,
    role: String,
    times: usize,
) -> Result<(), Unmet> {
    let called = Operation { role, operation };
    let found = world
        .witness
        .operations()
        .iter()
        .filter(|operation| is_called(operation, &called))
        .count();
    holds(
        found == times,
        format_args!("{called:?} called {times} times"),
        format_args!("{found} calls"),
    )
}

fn is_called(operation: &&Operation, called: &Operation) -> bool {
    *operation == called
}

#[then(
    expr = "the result names the step {string} whose hook failed the journey, with the code {string}"
)]
fn the_result_names_the_step_whose_hook_failed_the_journey(
    world: &mut World,
    step_name: String,
    code: String,
) -> Result<(), Unmet> {
    let found = match &result(world)?.status {
        JourneyStatus::Failed {
            failure: Failure::FailWorkflow(reason),
            ..
        } => Some(reason.code()),
        _ => None,
    };
    let (events, lines) = stream(world)?;
    expect(
        found == Some(code.as_str()) && lines.iter().any(|line| fails_at(line, &step_name)),
        format_args!(
            "a FailWorkflow with the code \"{code}\", whose journey_failed names \"{step_name}\""
        ),
        &events,
    )
}

#[then(
    expr = "the contribution of {string} is recorded as made by the hook {string} of policy {string}"
)]
fn the_contribution_is_recorded_as_made_by_the_hook(
    world: &mut World,
    key: String,
    hook: String,
    policy: String,
) -> Result<(), Unmet> {
    let (events, lines) = stream(world)?;
    let source = format!("{policy}, {hook}");
    expect(
        lines.iter().any(|line| commits_from(line, &key, &source)),
        format_args!("a contribution_committed of \"{key}\" from {source}"),
        &events,
    )
}

fn commits_from(line: &Line, key: &str, source: &str) -> bool {
    line.event == "contribution_committed"
        && line.has_cell("key", key)
        && line.has_cell("source", source)
}
