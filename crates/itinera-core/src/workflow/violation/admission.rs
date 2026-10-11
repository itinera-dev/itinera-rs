//! Admission's checks on a workflow's declaration, which find every violation it has.

use std::collections::BTreeMap;

use super::{Violation, Violations};
use crate::policy::{PolicyName, StepHook, StepPolicyEntry, WorkflowHook, WorkflowPolicyEntry};
use crate::step::{StepDescriptor, StepName};
use crate::workflow::{AdapterName, InputAdapterDescriptor};

/// Checks a workflow's declaration, and returns every violation found.
pub(crate) fn check<M, W>(
    steps: &[StepDescriptor<W, M>],
    policies: &[Box<dyn WorkflowPolicyEntry<W, M>>],
    adapters: &[InputAdapterDescriptor<W>],
) -> Result<(), Violations> {
    let violations: Vec<Violation> = duplicate_step_names(steps)
        .chain(steps.iter().flat_map(step_hooks_defined_twice))
        .chain(workflow_hooks_defined_twice(policies))
        .chain(adapters_for_unknown_steps(steps, adapters))
        .chain(steps_adapted_twice(steps, adapters))
        .collect();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(Violations { violations })
    }
}

fn duplicate_step_names<W, M>(steps: &[StepDescriptor<W, M>]) -> impl Iterator<Item = Violation> {
    counted(steps.iter().map(StepDescriptor::name))
        .into_iter()
        .filter(repeated)
        .map(duplicate_step_name)
}

fn duplicate_step_name((step, _): (StepName, usize)) -> Violation {
    Violation::DuplicateStepName { step }
}

fn step_hooks_defined_twice<W, M>(step: &StepDescriptor<W, M>) -> impl Iterator<Item = Violation> {
    let name = step.name();
    let definitions = step
        .policies()
        .iter()
        .map(Box::as_ref)
        .flat_map(step_hook_definitions);
    grouped(definitions)
        .into_iter()
        .filter(more_than_once)
        .map(move |(hook, policies)| step_hook_defined_twice(name, hook, policies))
}

fn step_hook_definitions<W, M>(
    policy: &dyn StepPolicyEntry<W, M>,
) -> impl Iterator<Item = (StepHook, PolicyName)> + '_ {
    let name = policy.name();
    policy
        .hooks()
        .iter()
        .map(move |hook| defined_by(*hook, name))
}

fn step_hook_defined_twice(step: StepName, hook: StepHook, policies: Vec<PolicyName>) -> Violation {
    Violation::StepHookDefinedTwice {
        step,
        hook,
        policies,
    }
}

fn workflow_hooks_defined_twice<W, M>(
    policies: &[Box<dyn WorkflowPolicyEntry<W, M>>],
) -> impl Iterator<Item = Violation> {
    grouped(
        policies
            .iter()
            .map(Box::as_ref)
            .flat_map(workflow_hook_definitions),
    )
    .into_iter()
    .filter(more_than_once)
    .map(workflow_hook_defined_twice)
}

fn workflow_hook_definitions<W, M>(
    policy: &dyn WorkflowPolicyEntry<W, M>,
) -> impl Iterator<Item = (WorkflowHook, PolicyName)> + '_ {
    let name = policy.name();
    policy
        .hooks()
        .iter()
        .map(move |hook| defined_by(*hook, name))
}

fn workflow_hook_defined_twice((hook, policies): (WorkflowHook, Vec<PolicyName>)) -> Violation {
    Violation::WorkflowHookDefinedTwice { hook, policies }
}

fn defined_by<H>(hook: H, policy: PolicyName) -> (H, PolicyName) {
    (hook, policy)
}

fn adapters_for_unknown_steps<'a, M, W>(
    steps: &'a [StepDescriptor<W, M>],
    adapters: &'a [InputAdapterDescriptor<W>],
) -> impl Iterator<Item = Violation> + 'a {
    adapters
        .iter()
        .flat_map(attachments)
        .filter(move |(_, step)| is_unknown(steps, *step))
        .map(adapter_for_unknown_step)
}

fn adapter_for_unknown_step((adapter, step): (AdapterName, StepName)) -> Violation {
    Violation::InputAdapterForUnknownStep { adapter, step }
}

fn steps_adapted_twice<M, W>(
    steps: &[StepDescriptor<W, M>],
    adapters: &[InputAdapterDescriptor<W>],
) -> impl Iterator<Item = Violation> {
    let known = adapters
        .iter()
        .flat_map(attachments)
        .filter(|(_, step)| is_known(steps, *step))
        .map(by_step);
    grouped(known)
        .into_iter()
        .filter(more_than_once)
        .map(step_adapted_twice)
}

fn step_adapted_twice((step, adapters): (StepName, Vec<AdapterName>)) -> Violation {
    Violation::StepAdaptedTwice { step, adapters }
}

/// Each step the adapter is attached to, with the adapter's name.
fn attachments<W>(
    adapter: &InputAdapterDescriptor<W>,
) -> impl Iterator<Item = (AdapterName, StepName)> + '_ {
    let name = adapter.name();
    adapter
        .steps()
        .iter()
        .map(move |step| attached(name, *step))
}

fn attached(adapter: AdapterName, step: StepName) -> (AdapterName, StepName) {
    (adapter, step)
}

fn by_step((adapter, step): (AdapterName, StepName)) -> (StepName, AdapterName) {
    (step, adapter)
}

fn is_known<W, M>(steps: &[StepDescriptor<W, M>], name: StepName) -> bool {
    steps.iter().any(|step| step.is_named(name))
}

fn is_unknown<W, M>(steps: &[StepDescriptor<W, M>], name: StepName) -> bool {
    !is_known(steps, name)
}

/// Groups values by key, keeping the values of each key in order.
fn grouped<K: Ord, V>(pairs: impl Iterator<Item = (K, V)>) -> BTreeMap<K, Vec<V>> {
    pairs.fold(BTreeMap::new(), add_to_group)
}

fn add_to_group<K: Ord, V>(
    mut groups: BTreeMap<K, Vec<V>>,
    (key, value): (K, V),
) -> BTreeMap<K, Vec<V>> {
    groups.entry(key).or_default().push(value);
    groups
}

fn more_than_once<K, V>((_, values): &(K, Vec<V>)) -> bool {
    values.len() > 1
}

/// Counts how many times each key occurs.
fn counted<K: Ord>(keys: impl Iterator<Item = K>) -> BTreeMap<K, usize> {
    keys.fold(BTreeMap::new(), add_to_count)
}

fn add_to_count<K: Ord>(mut counts: BTreeMap<K, usize>, key: K) -> BTreeMap<K, usize> {
    *counts.entry(key).or_default() += 1;
    counts
}

fn repeated<K>((_, count): &(K, usize)) -> bool {
    *count > 1
}
