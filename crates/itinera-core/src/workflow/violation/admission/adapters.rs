//! Admission's checks on the input adapters a workflow attaches to its steps.

use super::{counted, grouped, is_known, is_unknown, more_than_once, repeated};
use crate::step::{StepDescriptor, StepName};
use crate::workflow::{AdapterName, InputAdapterDescriptor, Violation};

pub(super) fn duplicate_adapter_names<W>(
    adapters: &[InputAdapterDescriptor<W>],
) -> impl Iterator<Item = Violation> {
    counted(adapters.iter().map(InputAdapterDescriptor::name))
        .into_iter()
        .filter(repeated)
        .map(duplicate_adapter_name)
}

fn duplicate_adapter_name((adapter, _): (AdapterName, usize)) -> Violation {
    Violation::DuplicateAdapterName { adapter }
}

pub(super) fn adapters_for_unknown_steps<'a, M, W>(
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

pub(super) fn steps_adapted_twice<M, W>(
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::step::RescueOutcome;
    use crate::workflow::WorkflowBuilder;
    use crate::workflow::fixtures::{Orders, adapter, orders, step};

    fn violations(builder: WorkflowBuilder<Orders>) -> Vec<Violation> {
        builder.build().unwrap_err().into_iter().collect()
    }

    fn rescued_charge() -> StepDescriptor<Orders> {
        step("charge").rescue(|| Ok(RescueOutcome::success()))
    }

    #[rstest]
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
    #[case::one_adapter_on_one_step_twice(
        orders()
            .step(step("charge"))
            .input_adapter(adapter("pricing", "charge").step(StepName::new("charge"))),
        Violation::StepAdaptedTwice {
            step: StepName::new("charge"),
            adapters: vec![AdapterName::from("pricing"), AdapterName::from("pricing")],
        },
    )]
    #[case::two_adapters_with_one_name(
        orders()
            .step(step("charge"))
            .step(step("refund"))
            .input_adapter(adapter("pricing", "charge"))
            .input_adapter(adapter("pricing", "refund")),
        Violation::DuplicateAdapterName { adapter: AdapterName::from("pricing") },
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
    #[case::two_adapters_on_one_rescue_step(
        orders()
            .step(rescued_charge())
            .input_adapter(adapter("credentials", "charge_rescue"))
            .input_adapter(adapter("accounts", "charge_rescue")),
        Violation::StepAdaptedTwice {
            step: StepName::new("charge_rescue"),
            adapters: vec![AdapterName::from("credentials"), AdapterName::from("accounts")],
        },
    )]
    #[case::one_adapter_on_one_rescue_step_twice(
        orders()
            .step(rescued_charge())
            .input_adapter(
                adapter("credentials", "charge_rescue").step(StepName::new("charge_rescue")),
            ),
        Violation::StepAdaptedTwice {
            step: StepName::new("charge_rescue"),
            adapters: vec![AdapterName::from("credentials"), AdapterName::from("credentials")],
        },
    )]
    #[case::an_adapter_on_the_rescue_step_of_a_step_without_one(
        orders()
            .step(step("charge"))
            .input_adapter(adapter("credentials", "charge_rescue")),
        Violation::InputAdapterForUnknownStep {
            adapter: AdapterName::from("credentials"),
            step: StepName::new("charge_rescue"),
        },
    )]
    fn a_workflow_whose_input_adapters_are_declared_wrongly_is_refused_with_its_violation(
        #[case] builder: WorkflowBuilder<Orders>,
        #[case] violation: Violation,
    ) {
        assert_eq!(violations(builder), [violation]);
    }

    #[test]
    fn an_adapter_may_be_attached_to_a_step_and_to_its_rescue_step() {
        let builder = orders()
            .step(rescued_charge())
            .input_adapter(adapter("credentials", "charge").step(StepName::new("charge_rescue")));
        assert!(builder.build().is_ok());
    }
}
