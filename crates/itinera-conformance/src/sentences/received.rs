//! Outcomes: what each step was built with, and what each hook received, as the witness of the
//! scenario recorded them.

mod hooks;
mod steps;

use crate::model::{Hook, ModelError};
use crate::witness::Call;
use crate::world::World;

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
