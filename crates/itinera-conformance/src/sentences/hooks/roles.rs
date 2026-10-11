//! Roles: the roles a workflow provides, the hooks that call them, and their calls.

use cucumber::{given, then};

use crate::model::{HookAction, HookRequest, ModelError};
use crate::sentences::{Unmet, holds};
use crate::witness::Operation;
use crate::world::World;

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
