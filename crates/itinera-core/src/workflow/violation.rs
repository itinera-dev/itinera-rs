use super::AdapterName;
use crate::policy::{PolicyName, StepHook, WorkflowHook};
use crate::step::StepName;

mod admission;

pub(super) use admission::check;

/// Every violation found in a workflow's declaration, at least one, which kept it from being
/// built.
///
/// # Examples
///
/// ```
/// use itinera::workflow::Violations;
///
/// fn report(violations: &Violations) -> Vec<String> {
///     violations.iter().map(ToString::to_string).collect()
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::IntoIterator)]
#[error("the workflow's declaration has violations: {}", listed(.violations))]
#[into_iterator(owned, ref)]
pub struct Violations {
    violations: Vec<Violation>,
}

impl Violations {
    /// The violations, in a fixed order.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::{Violation, Violations};
    ///
    /// fn first(violations: &Violations) -> Option<&Violation> {
    ///     violations.iter().next()
    /// }
    /// ```
    pub fn iter(&self) -> std::slice::Iter<'_, Violation> {
        self.violations.iter()
    }
}

/// One configuration error in how a workflow is put together, found when it is built.
///
/// It displays as the violation is named, followed by what it concerns, for example
/// `duplicate step name: charge`.
///
/// # Examples
///
/// ```
/// use itinera::workflow::{Violation, ViolationKind};
///
/// fn about_steps(violation: &Violation) -> bool {
///     violation.kind() == ViolationKind::DuplicateStepName
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, derive_more::Display)]
#[non_exhaustive]
pub enum Violation {
    /// Two or more steps have this name.
    #[display("duplicate step name: {step}")]
    #[non_exhaustive]
    DuplicateStepName {
        /// The name the steps share.
        step: StepName,
    },
    /// Two or more step policies attached to one step define the same hook.
    #[display("hook defined twice: {hook} on step {step}")]
    #[non_exhaustive]
    StepHookDefinedTwice {
        /// The step the policies are attached to.
        step: StepName,
        /// The hook they all define.
        hook: StepHook,
        /// The policies that define it, in the order they are attached.
        policies: Vec<PolicyName>,
    },
    /// Two or more workflow policies define the same hook.
    #[display("hook defined twice: {hook}")]
    #[non_exhaustive]
    WorkflowHookDefinedTwice {
        /// The hook they all define.
        hook: WorkflowHook,
        /// The policies that define it, in the order they are attached.
        policies: Vec<PolicyName>,
    },
    /// Two or more input adapters are attached to one step.
    #[display("step adapted twice: {step}")]
    #[non_exhaustive]
    StepAdaptedTwice {
        /// The step they are attached to.
        step: StepName,
        /// The adapters, in the order they were declared.
        adapters: Vec<AdapterName>,
    },
    /// An input adapter is attached to a step the workflow does not have.
    #[display("input adapter for unknown step: {adapter} on {step}")]
    #[non_exhaustive]
    InputAdapterForUnknownStep {
        /// The adapter.
        adapter: AdapterName,
        /// The name it gives for a step, which no step of the workflow has.
        step: StepName,
    },
}

impl Violation {
    /// The violation's kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::{Violation, ViolationKind};
    ///
    /// fn about_hooks(violation: &Violation) -> bool {
    ///     violation.kind() == ViolationKind::HookDefinedTwice
    /// }
    /// ```
    pub fn kind(&self) -> ViolationKind {
        match self {
            Self::DuplicateStepName { .. } => ViolationKind::DuplicateStepName,
            Self::StepHookDefinedTwice { .. } | Self::WorkflowHookDefinedTwice { .. } => {
                ViolationKind::HookDefinedTwice
            }
            Self::StepAdaptedTwice { .. } => ViolationKind::StepAdaptedTwice,
            Self::InputAdapterForUnknownStep { .. } => ViolationKind::InputAdapterForUnknownStep,
        }
    }
}

/// The kinds of violation a workflow can have when it is built.
///
/// It displays as the specification names the violation, for example `duplicate step name`. A
/// policy that needs a role the workflow does not provide, and a part in a mode the executor does
/// not accept, do not compile, so they have no kind here.
///
/// # Examples
///
/// ```
/// use itinera::workflow::ViolationKind;
///
/// assert_eq!(ViolationKind::StepAdaptedTwice.to_string(), "step adapted twice");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum ViolationKind {
    /// Two policies attached to the same step, or to the workflow, define the same hook.
    #[display("hook defined twice")]
    HookDefinedTwice,
    /// Two steps have the same name.
    #[display("duplicate step name")]
    DuplicateStepName,
    /// Two input adapters are attached to the same step.
    #[display("step adapted twice")]
    StepAdaptedTwice,
    /// An input adapter is attached to a step the workflow does not have.
    #[display("input adapter for unknown step")]
    InputAdapterForUnknownStep,
}

fn listed(violations: &[Violation]) -> String {
    violations
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::policy::tests::Quiet;
    use crate::policy::{StepPolicyDescriptor, WorkflowPolicyDescriptor};
    use crate::step::{Outcome, StepDescriptor};
    use crate::workflow::{InputAdapterDescriptor, WorkflowBuilder, WorkflowDescriptor};

    struct Orders;

    fn step(name: &'static str) -> StepDescriptor<Orders> {
        StepDescriptor::new(StepName::new(name), || Ok(Outcome::success()))
    }

    fn adapter(name: &'static str, step: &'static str) -> InputAdapterDescriptor<Orders> {
        InputAdapterDescriptor::new(name, StepName::new(step), |_, _| Ok(None))
    }

    fn orders() -> WorkflowBuilder<Orders> {
        WorkflowDescriptor::builder("orders")
    }

    fn audit() -> StepPolicyDescriptor<Quiet, Orders> {
        StepPolicyDescriptor::new("audit", || Quiet).on_step_success()
    }

    fn alarm() -> StepPolicyDescriptor<Quiet, Orders> {
        StepPolicyDescriptor::new("alarm", || Quiet).on_step_failure()
    }

    fn notify() -> WorkflowPolicyDescriptor<Quiet, Orders> {
        WorkflowPolicyDescriptor::new("notify", || Quiet).on_workflow_failure()
    }

    fn violations(builder: WorkflowBuilder<Orders>) -> Vec<Violation> {
        builder.build().unwrap_err().into_iter().collect()
    }

    #[rstest]
    #[case::two_steps_with_one_name(
        orders().step(step("charge")).step(step("charge")),
        Violation::DuplicateStepName { step: StepName::new("charge") },
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

    #[test]
    #[should_panic(expected = "the workflow already has an input adapter named \"pricing\"")]
    fn two_input_adapters_with_one_name_cannot_be_declared() {
        let _ = orders()
            .input_adapter(adapter("pricing", "charge"))
            .input_adapter(adapter("pricing", "ship"));
    }

    #[test]
    fn violations_display_every_violation() {
        let builder = orders()
            .step(step("charge"))
            .step(step("charge"))
            .input_adapter(adapter("pricing", "refund"));
        assert_eq!(
            builder.build().unwrap_err().to_string(),
            "the workflow's declaration has violations: duplicate step name: charge; \
             input adapter for unknown step: pricing on refund"
        );
    }

    #[rstest]
    #[case::hook_defined_twice(ViolationKind::HookDefinedTwice, "hook defined twice")]
    #[case::duplicate_step_name(ViolationKind::DuplicateStepName, "duplicate step name")]
    #[case::step_adapted_twice(ViolationKind::StepAdaptedTwice, "step adapted twice")]
    #[case::input_adapter_for_unknown_step(
        ViolationKind::InputAdapterForUnknownStep,
        "input adapter for unknown step"
    )]
    fn violation_kinds_display_as_the_specification_names_them(
        #[case] kind: ViolationKind,
        #[case] name: &str,
    ) {
        assert_eq!(kind.to_string(), name);
    }
}
