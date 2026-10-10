//! Policies: their descriptors, the hooks they define, what hooks request, the lifecycles they
//! return, and the roles through which they reach a workflow.

use crate::step::Reason;

#[cfg(feature = "async")]
mod asynchronous;
mod descriptor;
mod hook;
mod kind;
mod needs;
mod reporter;

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
pub(crate) use needs::{Answers, Needs, Request};
pub use needs::{HookNeeds, Requested};
pub use reporter::HookReporter;

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

/// What `on step success` may return: a lifecycle that ends the journey at once.
///
/// Returning `None` instead goes on to the next step, or succeeds the journey after the last.
///
/// # Examples
///
/// ```
/// use itinera::policy::OnSuccess;
/// use itinera::step::Reason;
///
/// fn ends_without_the_steps_left(returned: &OnSuccess) -> bool {
///     matches!(returned, OnSuccess::FinishWorkflow)
/// }
///
/// assert!(ends_without_the_steps_left(&OnSuccess::FinishWorkflow));
/// assert!(!ends_without_the_steps_left(&OnSuccess::FailWorkflow(Reason::new("refused"))));
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum OnSuccess {
    /// The journey succeeds at once, and the steps left are not run.
    FinishWorkflow,
    /// The journey fails at once, with this reason. The step stays succeeded.
    FailWorkflow(Reason),
}

/// The lifecycle `on step failure`, `on step retry` and `on step abnormal termination` may
/// return: the journey fails at once, with this reason.
///
/// Returning `None` instead keeps the default: the journey fails with the step's own failure
/// after `on step failure`, and the step is tried again or given up after the two others.
///
/// # Examples
///
/// ```
/// use itinera::policy::FailWorkflow;
/// use itinera::step::Reason;
///
/// let fail = FailWorkflow::from(Reason::new("fraud").with_message("the card is blocked"));
/// let reason: Reason = fail.into();
/// assert_eq!(reason.code(), "fraud");
/// ```
#[derive(Clone, Debug, derive_more::From, derive_more::Into)]
pub struct FailWorkflow(Reason);

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

/// A role the workflow's own type provides: the operations of the trait `R`, which policies
/// reach through [`Requested::role`].
///
/// A workflow implements it once for each role, returning itself. A policy whose hooks request a
/// role is implemented only for workflows that provide it, so attaching it to a workflow without
/// the role does not compile.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::Provides;
///
/// trait Notifier {
///     fn notify(&self, message: &str) -> Result<(), Error>;
/// }
///
/// struct Orders;
///
/// impl Notifier for Orders {
///     fn notify(&self, _message: &str) -> Result<(), Error> {
///         Ok(())
///     }
/// }
///
/// impl Provides<dyn Notifier> for Orders {
///     fn role(&self) -> &(dyn Notifier + 'static) {
///         self
///     }
/// }
///
/// Orders.role().notify("order shipped")?;
/// # Ok::<(), Error>(())
/// ```
#[diagnostic::on_unimplemented(
    message = "the workflow `{Self}` does not provide the role `{R}`",
    label = "this workflow does not provide the role",
    note = "a workflow provides a role by implementing `Provides<{R}>`"
)]
pub trait Provides<R: ?Sized> {
    /// The workflow, as the role.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::Provides;
    ///
    /// trait Clock {
    ///     fn hour(&self) -> u8;
    /// }
    ///
    /// struct Orders;
    ///
    /// impl Clock for Orders {
    ///     fn hour(&self) -> u8 {
    ///         9
    ///     }
    /// }
    ///
    /// impl Provides<dyn Clock> for Orders {
    ///     fn role(&self) -> &(dyn Clock + 'static) {
    ///         self
    ///     }
    /// }
    ///
    /// let clock: &dyn Clock = Orders.role();
    /// assert_eq!(clock.hour(), 9);
    /// ```
    fn role(&self) -> &R;
}

impl From<OnSuccess> for Lifecycle {
    fn from(returned: OnSuccess) -> Self {
        match returned {
            OnSuccess::FinishWorkflow => Self::FinishWorkflow,
            OnSuccess::FailWorkflow(reason) => Self::FailWorkflow(reason),
        }
    }
}

impl From<FailWorkflow> for Lifecycle {
    fn from(returned: FailWorkflow) -> Self {
        Self::FailWorkflow(returned.into())
    }
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
