//! Policies: the hooks they define, and the lifecycles hooks return.

use crate::step::Reason;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
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
