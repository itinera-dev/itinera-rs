//! Admission's checks on a workflow's declaration, which find every violation it has.

use super::{Violation, Violations};
use crate::policy::{PolicyName, StepHook, StepPolicyEntry, WorkflowHook, WorkflowPolicyEntry};
use crate::step::{StepDescriptor, StepName};
use crate::workflow::{AdapterName, InputAdapterDescriptor};
use tally::{counted, grouped, more_than_once, repeated};

mod tally;

/// Checks a workflow's declaration, and returns every violation found.
pub(crate) fn check<M, W>(
    steps: &[StepDescriptor<W, M>],
    policies: &[Box<dyn WorkflowPolicyEntry<W, M>>],
    adapters: &[InputAdapterDescriptor<W>],
) -> Result<(), Violations> {
    let violations: Vec<Violation> = duplicate_step_names(steps)
        .chain(rescued_steps_retried(steps))
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
    counted(steps.iter().flat_map(names))
        .into_iter()
        .filter(repeated)
        .map(duplicate_step_name)
}

/// The step's name, then its rescue step's, if it has one.
fn names<W, M>(step: &StepDescriptor<W, M>) -> impl Iterator<Item = StepName> {
    std::iter::once(step.name()).chain(step.rescue_step().map(StepDescriptor::name))
}

fn duplicate_step_name((step, _): (StepName, usize)) -> Violation {
    Violation::DuplicateStepName { step }
}

fn rescued_steps_retried<W, M>(
    steps: &[StepDescriptor<W, M>],
) -> impl Iterator<Item = Violation> + '_ {
    steps
        .iter()
        .filter(rescued_and_retried)
        .map(rescued_step_retried)
}

fn rescued_and_retried<W, M>(step: &&StepDescriptor<W, M>) -> bool {
    step.rescue_step().is_some() && step.allows_retries()
}

fn rescued_step_retried<W, M>(step: &StepDescriptor<W, M>) -> Violation {
    Violation::RescuedStepRetried { step: step.name() }
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::policy::fixtures::Quiet;
    use crate::policy::{StepPolicyDescriptor, WorkflowPolicyDescriptor};
    use crate::step::RescueOutcome;
    use crate::workflow::fixtures::{Orders, adapter, orders, step};
    use crate::workflow::{ViolationKind, WorkflowBuilder};

    fn audit() -> StepPolicyDescriptor<Quiet, Orders> {
        StepPolicyDescriptor::new("audit", || Quiet).on_step_success()
    }

    fn alarm() -> StepPolicyDescriptor<Quiet, Orders> {
        StepPolicyDescriptor::new("alarm", || Quiet).on_step_failure()
    }

    fn notify() -> WorkflowPolicyDescriptor<Quiet, Orders> {
        WorkflowPolicyDescriptor::new("notify", || Quiet).on_workflow_failure()
    }

    fn rescued_charge() -> StepDescriptor<Orders> {
        step("charge").rescue(|| Ok(RescueOutcome::success()))
    }

    fn violations(builder: WorkflowBuilder<Orders>) -> Vec<Violation> {
        builder.build().unwrap_err().into_iter().collect()
    }

    #[rstest]
    #[case::two_steps_with_one_name(
        orders().step(step("charge")).step(step("charge")),
        Violation::DuplicateStepName { step: StepName::new("charge") },
    )]
    #[case::a_step_named_as_another_steps_rescue_step(
        orders()
            .step(rescued_charge())
            .step(step("charge_rescue")),
        Violation::DuplicateStepName { step: StepName::new("charge_rescue") },
    )]
    #[case::a_step_with_a_rescue_step_and_a_retry_budget(
        orders().step(rescued_charge().retry_budget(1)),
        Violation::RescuedStepRetried { step: StepName::new("charge") },
    )]
    #[case::an_adapter_on_a_rescue_step(
        orders()
            .step(rescued_charge())
            .input_adapter(adapter("pricing", "charge_rescue")),
        Violation::InputAdapterForUnknownStep {
            adapter: AdapterName::from("pricing"),
            step: StepName::new("charge_rescue"),
        },
    )]
    #[case::two_step_policies_defining_one_hook(
        orders().step(step("charge").policy(audit()).policy(
            StepPolicyDescriptor::new("metrics", || Quiet)
                .on_step_failure()
                .on_step_success(),
        )),
        Violation::StepHookDefinedTwice {
            step: StepName::new("charge"),
            hook: StepHook::OnStepSuccess,
            policies: vec![PolicyName::from("audit"), PolicyName::from("metrics")],
        },
    )]
    #[case::one_step_policy_attached_twice(
        orders().step(step("charge").policy(audit()).policy(audit())),
        Violation::StepHookDefinedTwice {
            step: StepName::new("charge"),
            hook: StepHook::OnStepSuccess,
            policies: vec![PolicyName::from("audit"), PolicyName::from("audit")],
        },
    )]
    #[case::two_workflow_policies_defining_one_hook(
        orders()
            .policy(notify())
            .policy(WorkflowPolicyDescriptor::new("close", || Quiet).on_workflow_failure()),
        Violation::WorkflowHookDefinedTwice {
            hook: WorkflowHook::OnWorkflowFailure,
            policies: vec![PolicyName::from("notify"), PolicyName::from("close")],
        },
    )]
    #[case::two_adapters_on_one_step(
        orders()
            .step(step("charge"))
            .input_adapter(adapter("pricing", "charge"))
            .input_adapter(adapter("discounts", "charge")),
        Violation::StepAdaptedTwice {
            step: StepName::new("charge"),
            adapters: vec![AdapterName::from("pricing"), AdapterName::from("discounts")],
        },
    )]
    #[case::an_adapter_on_a_step_the_workflow_does_not_have(
        orders()
            .step(step("charge"))
            .input_adapter(adapter("pricing", "refund")),
        Violation::InputAdapterForUnknownStep {
            adapter: AdapterName::from("pricing"),
            step: StepName::new("refund"),
        },
    )]
    fn a_workflow_put_together_wrongly_is_refused_with_its_violation(
        #[case] builder: WorkflowBuilder<Orders>,
        #[case] violation: Violation,
    ) {
        assert_eq!(violations(builder), [violation]);
    }

    #[test]
    fn every_violation_is_reported_together() {
        let builder = orders()
            .step(step("charge").policy(audit()).policy(audit()))
            .step(step("charge"))
            .input_adapter(adapter("pricing", "refund"));
        let kinds: Vec<ViolationKind> = violations(builder).iter().map(Violation::kind).collect();
        assert_eq!(
            kinds,
            [
                ViolationKind::DuplicateStepName,
                ViolationKind::HookDefinedTwice,
                ViolationKind::InputAdapterForUnknownStep,
            ]
        );
    }

    #[test]
    fn a_workflow_put_together_rightly_is_built() {
        let builder = orders()
            .step(step("charge").policy(audit()).policy(alarm()))
            .step(step("Charge").policy(audit()))
            .policy(notify())
            .policy(WorkflowPolicyDescriptor::new("close", || Quiet).on_workflow_success())
            .input_adapter(
                adapter("pricing", "charge")
                    .step(StepName::new("Charge"))
                    .step(StepName::new("charge")),
            );
        assert!(builder.build().is_ok());
    }
}
