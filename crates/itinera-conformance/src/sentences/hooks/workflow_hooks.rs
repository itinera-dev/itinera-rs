//! Workflow hooks: what they return, which is never read, and how they run.

use cucumber::given;

use super::returns;
use crate::executor::Executor;
use crate::model::{Hook, HookReturn, ModelError};
use crate::world::World;

/// A workflow hook is void, and in Rust it returns only whether it failed, so it cannot return
/// a value: it returns nothing, as it does when no sentence says what it returns.
#[given(expr = "the hook {string} of policy {string} returns a value")]
fn the_hook_returns_a_value(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    match Hook::named(&hook)? {
        Hook::Workflow(_) => returns(world, &policy, &hook, HookReturn::Nothing),
        step_hook @ Hook::Step(_) => Err(ModelError::ResultOfStepHook(step_hook)),
    }
}

/// Every hook the runner declares for the asynchronous executor is asynchronous, so the sentence
/// only holds the scenario to that executor.
#[given(expr = "the hook {string} of policy {string} is asynchronous")]
fn the_hook_is_asynchronous(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), ModelError> {
    world.model.hook_mut(&policy, &hook)?;
    match world.executor {
        Some(Executor::AsyncLocal) => Ok(()),
        _ => Err(ModelError::AsynchronousHook(Hook::named(&hook)?)),
    }
}
