//! Policies: their descriptors, the hooks they define, what hooks request, the lifecycles they
//! return, and the roles through which they reach a workflow.

#[cfg(feature = "async")]
mod asynchronous;
mod descriptor;
mod hook;
mod kind;
mod lifecycle;
mod needs;
mod reporter;
mod role;

#[cfg(feature = "async")]
pub use asynchronous::{
    AsyncHookReporter, AsyncOnStepAbnormalTermination, AsyncOnStepFailure, AsyncOnStepRetry,
    AsyncOnStepSuccess, AsyncOnWorkflowFailure, AsyncOnWorkflowSuccess,
};
#[cfg(feature = "async")]
pub(crate) use descriptor::HookCall;
pub(crate) use descriptor::{
    BuiltStepPolicy, BuiltWorkflowPolicy, Call, StepPolicyEntry, WorkflowPolicyEntry,
};
pub use descriptor::{Hooked, Hookless, StepPolicyDescriptor, WorkflowPolicyDescriptor};
pub use hook::{
    OnStepAbnormalTermination, OnStepFailure, OnStepRetry, OnStepSuccess, OnWorkflowFailure,
    OnWorkflowSuccess,
};
pub use kind::{
    ErrorHookKind, FailureHookKind, HookKind, InputAdapter, PolicyHookKind,
    StepAbnormalTermination, StepFailure, StepHookKind, StepRetry, StepSuccess, WorkflowFailure,
    WorkflowSuccess,
};
pub use lifecycle::{FailWorkflow, Lifecycle, OnSuccess};
pub(crate) use needs::{Answers, Needs, Request};
pub use needs::{HookNeeds, Requested};
pub use reporter::HookReporter;
pub use role::Provides;

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

/// Why `on step failure` is called: why the step was given up.
///
/// It displays as the specification writes it, for example `retries exhausted`.
///
/// # Examples
///
/// ```
/// use itinera::policy::StepFailureCause;
///
/// assert_eq!(StepFailureCause::RetriesExhausted.to_string(), "retries exhausted");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum StepFailureCause {
    /// The attempt reported a failure that is not retriable.
    #[display("failure")]
    Failure,
    /// The attempt ended in an abnormal termination, which the step does not retry.
    #[display("abnormal termination")]
    AbnormalTermination,
    /// The attempt failed in a way that could be retried, but the retry budget was spent.
    #[display("retries exhausted")]
    RetriesExhausted,
}

/// Why `on step retry` is called: why the step will be attempted again.
///
/// It displays as the cause is named, for example `retriable failure`.
///
/// # Examples
///
/// ```
/// use itinera::policy::RetryCause;
///
/// assert_eq!(RetryCause::RetriableFailure.to_string(), "retriable failure");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, derive_more::Display)]
#[non_exhaustive]
pub enum RetryCause {
    /// The attempt reported a retriable failure.
    #[display("retriable failure")]
    RetriableFailure,
    /// The attempt ended in an abnormal termination, and the step allows retrying it.
    #[display("abnormal termination")]
    AbnormalTermination,
}

#[cfg(test)]
pub(crate) mod tests {
    use std::fmt;

    use rstest::rstest;

    use super::*;
    use crate::error::Error;
    use crate::step::Input;

    const RECEIPT: Input<String> = Input::new("receipt");

    /// A policy whose hooks change nothing.
    pub(crate) struct Quiet;

    impl<W: Send + Sync + 'static> OnStepSuccess<W> for Quiet {
        fn on_step_success(
            &self,
            _: Requested<'_, W, StepSuccess>,
        ) -> Result<Option<OnSuccess>, Error> {
            Ok(None)
        }
    }

    impl<W: Send + Sync + 'static> OnStepFailure<W> for Quiet {
        fn on_step_failure(
            &self,
            _: Requested<'_, W, StepFailure>,
        ) -> Result<Option<FailWorkflow>, Error> {
            Ok(None)
        }
    }

    impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Quiet {
        fn on_workflow_success(&self, _: Requested<'_, W, WorkflowSuccess>) -> Result<(), Error> {
            Ok(())
        }
    }

    impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Quiet {
        fn on_workflow_failure(&self, _: Requested<'_, W, WorkflowFailure>) -> Result<(), Error> {
            Ok(())
        }
    }

    #[cfg(feature = "async")]
    impl<W: Send + Sync + 'static> AsyncOnWorkflowSuccess<W> for Quiet {
        async fn on_workflow_success(
            &self,
            _: Requested<'_, W, WorkflowSuccess, crate::mode::Asynchronous>,
        ) -> Result<(), Error> {
            Ok(())
        }
    }

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
    #[case::failure(&StepFailureCause::Failure, "failure")]
    #[case::abnormal_termination(&StepFailureCause::AbnormalTermination, "abnormal termination")]
    #[case::retries_exhausted(&StepFailureCause::RetriesExhausted, "retries exhausted")]
    #[case::retry_after_a_retriable_failure(&RetryCause::RetriableFailure, "retriable failure")]
    #[case::retry_after_an_abnormal_termination(
        &RetryCause::AbnormalTermination,
        "abnormal termination"
    )]
    fn hooks_lifecycles_and_causes_display_as_the_specification_writes_them(
        #[case] name: &dyn fmt::Display,
        #[case] written: &str,
    ) {
        assert_eq!(name.to_string(), written);
    }

    /// The keys of the data a hook needs, from the step or from the workflow.
    fn keys(needs: &Needs) -> Vec<&'static str> {
        needs.requests().iter().filter_map(key).collect()
    }

    fn key(request: &Request) -> Option<&'static str> {
        match request {
            Request::FromStep(need) | Request::FromWorkflow(need) => Some(need.key()),
            Request::Reason(_) | Request::Error(_) => None,
        }
    }

    #[test]
    fn a_step_hook_named_twice_is_defined_once_with_the_needs_named_last() {
        let audit: StepPolicyDescriptor<Quiet, ()> = StepPolicyDescriptor::new("audit", || Quiet)
            .on_step_success()
            .on_step_success_needing(HookNeeds::new().from_step(&RECEIPT));

        assert_eq!(audit.hooks(), [StepHook::OnStepSuccess]);
        assert_eq!(keys(audit.needs(StepHook::OnStepSuccess)), ["receipt"]);
    }

    #[test]
    fn a_workflow_hook_named_twice_is_defined_once_with_the_needs_named_last() {
        let notify: WorkflowPolicyDescriptor<Quiet, ()> =
            WorkflowPolicyDescriptor::new("notify", || Quiet)
                .on_workflow_success_needing(HookNeeds::new().from_workflow(&RECEIPT))
                .on_workflow_success();

        assert_eq!(notify.hooks(), [WorkflowHook::OnWorkflowSuccess]);
        assert!(keys(notify.needs(WorkflowHook::OnWorkflowSuccess)).is_empty());
    }
}
