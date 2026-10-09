//! Policies: the hooks they define, and the lifecycles hooks return.

use crate::step::Reason;

/// A policy's name, fixed when the program is compiled.
///
/// # Examples
///
/// ```
/// use itinera::policy::PolicyName;
///
/// let audit = PolicyName::from("audit");
/// let text: &str = audit.as_ref();
/// assert_eq!(text, "audit");
/// assert_eq!(audit.to_string(), "audit");
/// ```
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
)]
#[as_ref(forward)]
pub struct PolicyName(&'static str);

/// The description of one step policy: its name and the step hooks it defines. It is attached to
/// steps with [`StepDescriptor::policy`], and may be attached to any number of them.
///
/// The hooks a policy defines are named here and keep their default behaviour, until hooks can be
/// written.
///
/// [`StepDescriptor::policy`]: crate::step::StepDescriptor::policy
///
/// # Examples
///
/// ```
/// use itinera::policy::{StepHook, StepPolicyDescriptor};
///
/// let audit = StepPolicyDescriptor::new("audit", StepHook::OnStepSuccess)
///     .hook(StepHook::OnStepFailure);
/// assert_eq!(audit.name().to_string(), "audit");
/// ```
#[derive(Clone, Debug)]
pub struct StepPolicyDescriptor {
    name: PolicyName,
    hooks: Vec<StepHook>,
}

impl StepPolicyDescriptor {
    /// Describes a step policy with its name and a step hook it defines.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{StepHook, StepPolicyDescriptor};
    ///
    /// let alarm = StepPolicyDescriptor::new("alarm", StepHook::OnStepFailure);
    /// # drop(alarm);
    /// ```
    pub fn new(name: impl Into<PolicyName>, hook: StepHook) -> Self {
        Self {
            name: name.into(),
            hooks: vec![hook],
        }
    }

    /// Adds a step hook the policy defines. A hook it already defines is not added again.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{StepHook, StepPolicyDescriptor};
    ///
    /// let retries = StepPolicyDescriptor::new("retries", StepHook::OnStepRetry)
    ///     .hook(StepHook::OnStepAbnormalTermination);
    /// # drop(retries);
    /// ```
    pub fn hook(mut self, hook: StepHook) -> Self {
        if !self.hooks.contains(&hook) {
            self.hooks.push(hook);
        }
        self
    }

    /// The policy's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{StepHook, StepPolicyDescriptor};
    ///
    /// let audit = StepPolicyDescriptor::new("audit", StepHook::OnStepSuccess);
    /// assert_eq!(audit.name().to_string(), "audit");
    /// ```
    pub fn name(&self) -> PolicyName {
        self.name
    }

    pub(crate) fn hooks(&self) -> &[StepHook] {
        &self.hooks
    }
}

/// The description of one workflow policy: its name and the workflow hooks it defines. It is
/// attached to workflows with [`WorkflowBuilder::policy`], and may be attached to any number of
/// them.
///
/// The hooks a policy defines are named here and keep their default behaviour, until hooks can be
/// written.
///
/// [`WorkflowBuilder::policy`]: crate::workflow::WorkflowBuilder::policy
///
/// # Examples
///
/// ```
/// use itinera::policy::{WorkflowHook, WorkflowPolicyDescriptor};
///
/// let notify = WorkflowPolicyDescriptor::new("notify", WorkflowHook::OnWorkflowSuccess)
///     .hook(WorkflowHook::OnWorkflowFailure);
/// assert_eq!(notify.name().to_string(), "notify");
/// ```
#[derive(Clone, Debug)]
pub struct WorkflowPolicyDescriptor {
    name: PolicyName,
    hooks: Vec<WorkflowHook>,
}

impl WorkflowPolicyDescriptor {
    /// Describes a workflow policy with its name and a workflow hook it defines.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{WorkflowHook, WorkflowPolicyDescriptor};
    ///
    /// let close = WorkflowPolicyDescriptor::new("close", WorkflowHook::OnWorkflowFailure);
    /// # drop(close);
    /// ```
    pub fn new(name: impl Into<PolicyName>, hook: WorkflowHook) -> Self {
        Self {
            name: name.into(),
            hooks: vec![hook],
        }
    }

    /// Adds a workflow hook the policy defines. A hook it already defines is not added again.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{WorkflowHook, WorkflowPolicyDescriptor};
    ///
    /// let notify = WorkflowPolicyDescriptor::new("notify", WorkflowHook::OnWorkflowSuccess)
    ///     .hook(WorkflowHook::OnWorkflowFailure);
    /// # drop(notify);
    /// ```
    pub fn hook(mut self, hook: WorkflowHook) -> Self {
        if !self.hooks.contains(&hook) {
            self.hooks.push(hook);
        }
        self
    }

    /// The policy's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{WorkflowHook, WorkflowPolicyDescriptor};
    ///
    /// let close = WorkflowPolicyDescriptor::new("close", WorkflowHook::OnWorkflowFailure);
    /// assert_eq!(close.name().to_string(), "close");
    /// ```
    pub fn name(&self) -> PolicyName {
        self.name
    }

    pub(crate) fn hooks(&self) -> &[WorkflowHook] {
        &self.hooks
    }
}

/// The name of a step hook, which acts on one attempt of a step.
///
/// It displays as the hook is named, for example `on step success`.
///
/// # Examples
///
/// ```
/// use itinera::policy::StepHook;
///
/// assert_eq!(StepHook::OnStepRetry.to_string(), "on step retry");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
#[non_exhaustive]
pub enum StepHook {
    /// Called after an attempt that succeeded.
    #[display("on step success")]
    OnStepSuccess,
    /// Called after a step was given up.
    #[display("on step failure")]
    OnStepFailure,
    /// Called before a step is attempted again.
    #[display("on step retry")]
    OnStepRetry,
    /// Called after an attempt ended in an abnormal termination.
    #[display("on step abnormal termination")]
    OnStepAbnormalTermination,
}

/// The name of a workflow hook, called once at the end of a journey.
///
/// It displays as the hook is named, for example `on workflow success`.
///
/// # Examples
///
/// ```
/// use itinera::policy::WorkflowHook;
///
/// assert_eq!(WorkflowHook::OnWorkflowFailure.to_string(), "on workflow failure");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, derive_more::Display)]
#[non_exhaustive]
pub enum WorkflowHook {
    /// Called once the journey has succeeded.
    #[display("on workflow success")]
    OnWorkflowSuccess,
    /// Called once the journey has failed.
    #[display("on workflow failure")]
    OnWorkflowFailure,
}

/// A lifecycle a hook returned, which decides what happens next.
///
/// It displays as the lifecycle is named, for example `FailWorkflow`.
///
/// # Examples
///
/// ```
/// use itinera::policy::Lifecycle;
///
/// fn ends_the_journey(lifecycle: &Lifecycle) -> bool {
///     matches!(lifecycle, Lifecycle::FinishWorkflow | Lifecycle::FailWorkflow(_))
/// }
///
/// assert_eq!(Lifecycle::FinishWorkflow.to_string(), "FinishWorkflow");
/// ```
#[derive(Clone, Debug, derive_more::Display)]
#[non_exhaustive]
pub enum Lifecycle {
    /// The journey succeeds at once.
    FinishWorkflow,
    /// The journey fails at once, with this reason.
    #[display("FailWorkflow")]
    FailWorkflow(Reason),
}

#[cfg(test)]
mod tests {
    use std::fmt;

    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::on_step_success(&StepHook::OnStepSuccess, "on step success")]
    #[case::on_step_failure(&StepHook::OnStepFailure, "on step failure")]
    #[case::on_step_retry(&StepHook::OnStepRetry, "on step retry")]
    #[case::on_step_abnormal_termination(
        &StepHook::OnStepAbnormalTermination,
        "on step abnormal termination"
    )]
    #[case::on_workflow_success(&WorkflowHook::OnWorkflowSuccess, "on workflow success")]
    #[case::on_workflow_failure(&WorkflowHook::OnWorkflowFailure, "on workflow failure")]
    #[case::finish_workflow(&Lifecycle::FinishWorkflow, "FinishWorkflow")]
    fn hooks_and_lifecycles_display_as_the_specification_writes_them(
        #[case] name: &dyn fmt::Display,
        #[case] written: &str,
    ) {
        assert_eq!(name.to_string(), written);
    }
}
