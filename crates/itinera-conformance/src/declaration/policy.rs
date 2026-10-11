//! The scenario's scripted policies, declared with the hooks their scripts define.

use itinera::policy::{Hooked, StepPolicyDescriptor, WorkflowPolicyDescriptor};

use super::{Declares, ScriptedWorkflow};
use crate::model::{HookScript, Hooks, Model, ModelError, Policy};
use crate::policy::{Building, ScriptedPolicy};
use crate::witness::Witness;

/// A scripted step policy, declared in the mode `M`, which names its hooks once it is `Hooked`.
pub(super) type StepPolicy<M, S = Hooked> =
    StepPolicyDescriptor<ScriptedPolicy, ScriptedWorkflow, M, S>;

/// A scripted workflow policy, declared in the mode `M`, which names its hooks once it is
/// `Hooked`.
pub(super) type WorkflowPolicy<M, S = Hooked> =
    WorkflowPolicyDescriptor<ScriptedPolicy, ScriptedWorkflow, M, S>;

pub(super) fn step_policy<M: Declares>(
    model: &Model,
    name: &str,
    witness: &Witness,
) -> Result<StepPolicy<M>, ModelError> {
    let policy = policy(model, name)?;
    let Hooks::Step(hooks) = &policy.hooks else {
        return Err(ModelError::NotStepPolicy(name.to_owned()));
    };
    let building = Building::of(name, policy, witness)?;
    let scripts = building.scripts();
    let mut hooks = hooks.iter().map(hook_of);
    let first = hooks.next().ok_or_else(|| no_hooks(name))?;
    let declared = M::step_hook(M::step_policy(building), first, &scripts)?;
    hooks.try_fold(declared, |declared, hook| {
        M::step_hook(declared, hook, &scripts)
    })
}

pub(super) fn workflow_policy<M: Declares>(
    model: &Model,
    name: &str,
    witness: &Witness,
) -> Result<WorkflowPolicy<M>, ModelError> {
    let policy = policy(model, name)?;
    let Hooks::Workflow(hooks) = &policy.hooks else {
        return Err(ModelError::NotWorkflowPolicy(name.to_owned()));
    };
    let building = Building::of(name, policy, witness)?;
    let scripts = building.scripts();
    let mut hooks = hooks.iter().map(hook_of);
    let first = hooks.next().ok_or_else(|| no_hooks(name))?;
    let declared = M::workflow_hook(M::workflow_policy(building), first, &scripts)?;
    hooks.try_fold(declared, |declared, hook| {
        M::workflow_hook(declared, hook, &scripts)
    })
}

fn policy<'a>(model: &'a Model, name: &str) -> Result<&'a Policy, ModelError> {
    model
        .policies
        .get(name)
        .ok_or_else(|| ModelError::UnknownPolicy(name.to_owned()))
}

/// A policy of the scenario that defines no hook, which itinera cannot declare.
fn no_hooks(name: &str) -> ModelError {
    ModelError::NoHooks(name.to_owned())
}

fn hook_of<H: Copy>((hook, _): &(H, HookScript)) -> H {
    *hook
}
