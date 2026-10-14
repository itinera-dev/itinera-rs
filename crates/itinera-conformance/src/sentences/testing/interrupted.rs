//! Which calls a step or a hook run alone made to its lent reporter were interrupted.

use cucumber::then;

use crate::model::{Hook, ModelError};
use crate::sentences::{Unmet, holds};
use crate::witness::{Caller, Reported};
use crate::world::World;

#[then(expr = "every call step {string} made to its step reporter was interrupted")]
fn every_call_the_step_made_was_interrupted(
    world: &mut World,
    step_name: String,
) -> Result<(), Unmet> {
    every_call_interrupted(world, &Caller::Step(step_name))
}

#[then(
    expr = "every call the hook {string} of policy {string} made to its hook reporter was interrupted"
)]
fn every_call_the_hook_made_was_interrupted(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), Unmet> {
    let hook = Hook::named(&hook)?;
    every_call_interrupted(world, &Caller::Hook { policy, hook })
}

fn every_call_interrupted(world: &World, caller: &Caller) -> Result<(), Unmet> {
    let calls = calls_of(world, caller)?;
    holds(
        !calls.is_empty() && calls.iter().all(is_interrupted),
        format_args!("every call {caller:?} made to its reporter to be interrupted"),
        format_args!("{calls:?}"),
    )
}

#[then(
    expr = "the call step {string} made setting {string}, and every call after it, was interrupted, and no call before it"
)]
fn the_call_setting_and_every_call_after_it_was_interrupted(
    world: &mut World,
    step_name: String,
    key: String,
) -> Result<(), Unmet> {
    let calls = calls_of(world, &Caller::Step(step_name.clone()))?;
    let setting = calls
        .iter()
        .position(|call| sets(call, &key))
        .ok_or_else(|| never_set(&step_name, &key, &calls))?;
    let (before, after) = calls.split_at(setting);
    holds(
        !before.iter().any(is_interrupted) && after.iter().all(is_interrupted),
        format_args!(
            "the call setting \"{key}\" and every call after it to be interrupted, and none before"
        ),
        format_args!("{calls:?}"),
    )
}

fn never_set(step: &str, key: &str, calls: &[Reported]) -> Unmet {
    Unmet::Expected(format!(
        "step \"{step}\" to set \"{key}\"; its calls were {calls:?}"
    ))
}

/// The calls this caller made to its reporter, in order.
fn calls_of(world: &World, caller: &Caller) -> Result<Vec<Reported>, ModelError> {
    Ok(world
        .witnessed()?
        .reports()
        .into_iter()
        .filter(|call| is_by(call, caller))
        .collect())
}

fn is_by(call: &Reported, caller: &Caller) -> bool {
    call.caller == *caller
}

fn is_interrupted(call: &Reported) -> bool {
    call.interrupted
}

fn sets(call: &Reported, key: &str) -> bool {
    call.setting.as_deref() == Some(key)
}
