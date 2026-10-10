//! The synchronous hooks a policy may define, one trait each.

use super::{
    FailWorkflow, HookNeeds, OnSuccess, Requested, StepAbnormalTermination, StepFailure, StepRetry,
    StepSuccess, WorkflowFailure, WorkflowSuccess,
};
use crate::error::Error;

/// `on step success`, called after each attempt of a step that succeeded, once its
/// contributions are committed. A step policy implements it for the workflows `W` it may be
/// attached to, and its descriptor declares it with
/// [`StepPolicyDescriptor::on_step_success`](super::StepPolicyDescriptor::on_step_success).
///
/// Returning `None` goes on to the next step; `FinishWorkflow` or `FailWorkflow` ends the journey
/// at once. An `Err` aborts the journey with `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{OnStepSuccess, OnSuccess, Requested, StepSuccess};
/// use itinera::step::StepName;
///
/// const CHARGE: StepName = StepName::new("charge");
///
/// struct StopAfterCharge;
///
/// impl<W: Send + Sync + 'static> OnStepSuccess<W> for StopAfterCharge {
///     fn on_step_success(
///         &self,
///         got: Requested<'_, W, StepSuccess>,
///     ) -> Result<Option<OnSuccess>, Error> {
///         Ok((got.step_name() == CHARGE).then_some(OnSuccess::FinishWorkflow))
///     }
/// }
/// ```
pub trait OnStepSuccess<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, OnStepSuccess, StepSuccess};
    ///
    /// fn needs_of<P: OnStepSuccess<()>>() -> HookNeeds<StepSuccess> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<StepSuccess> {
        HookNeeds::new()
    }

    /// Acts on the attempt that succeeded.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{OnStepSuccess, OnSuccess, Requested, StepSuccess};
    /// use itinera::step::Reason;
    ///
    /// struct OnlyOnce;
    ///
    /// impl<W: Send + Sync + 'static> OnStepSuccess<W> for OnlyOnce {
    ///     fn on_step_success(
    ///         &self,
    ///         got: Requested<'_, W, StepSuccess>,
    ///     ) -> Result<Option<OnSuccess>, Error> {
    ///         if got.attempt().get() > 1 {
    ///             return Ok(Some(OnSuccess::FailWorkflow(Reason::new("not idempotent"))));
    ///         }
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    fn on_step_success(
        &self,
        got: Requested<'_, W, StepSuccess>,
    ) -> Result<Option<OnSuccess>, Error>;
}

/// `on step failure`, called after a step was given up, once `step_given_up` is emitted. It
/// decides only the journey's fate, which fails either way.
///
/// Returning `None` fails the journey with the step's own failure; `FailWorkflow` fails it with
/// its own reason. An `Err` aborts the journey with `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{FailWorkflow, OnStepFailure, Requested, StepFailure};
/// use itinera::step::Reason;
///
/// struct Rename;
///
/// impl<W: Send + Sync + 'static> OnStepFailure<W> for Rename {
///     fn on_step_failure(
///         &self,
///         _got: Requested<'_, W, StepFailure>,
///     ) -> Result<Option<FailWorkflow>, Error> {
///         Ok(Some(FailWorkflow::from(Reason::new("order failed"))))
///     }
/// }
/// ```
pub trait OnStepFailure<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, OnStepFailure, StepFailure};
    ///
    /// fn needs_of<P: OnStepFailure<()>>() -> HookNeeds<StepFailure> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<StepFailure> {
        HookNeeds::new()
    }

    /// Acts on the step that was given up.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{FailWorkflow, OnStepFailure, Requested, StepFailure};
    ///
    /// struct Log;
    ///
    /// impl<W: Send + Sync + 'static> OnStepFailure<W> for Log {
    ///     fn on_step_failure(
    ///         &self,
    ///         got: Requested<'_, W, StepFailure>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         println!("{} was given up: {}", got.step_name(), got.cause());
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    fn on_step_failure(
        &self,
        got: Requested<'_, W, StepFailure>,
    ) -> Result<Option<FailWorkflow>, Error>;
}

/// `on step retry`, called when a step will be attempted again, before `step_retrying`.
///
/// Returning `None` lets the step be attempted again; `FailWorkflow` gives the step up and fails
/// the journey with its reason, without calling `on step failure`. An `Err` aborts the journey
/// with `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{FailWorkflow, OnStepRetry, Requested, StepRetry};
/// use itinera::step::Reason;
///
/// struct AtMostTwice;
///
/// impl<W: Send + Sync + 'static> OnStepRetry<W> for AtMostTwice {
///     fn on_step_retry(
///         &self,
///         got: Requested<'_, W, StepRetry>,
///     ) -> Result<Option<FailWorkflow>, Error> {
///         Ok((got.attempt().get() >= 2)
///             .then(|| FailWorkflow::from(Reason::new("too many tries"))))
///     }
/// }
/// ```
pub trait OnStepRetry<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, OnStepRetry, StepRetry};
    ///
    /// fn needs_of<P: OnStepRetry<()>>() -> HookNeeds<StepRetry> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<StepRetry> {
        HookNeeds::new()
    }

    /// Acts before the step is attempted again.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{FailWorkflow, OnStepRetry, Requested, StepRetry};
    ///
    /// struct Log;
    ///
    /// impl<W: Send + Sync + 'static> OnStepRetry<W> for Log {
    ///     fn on_step_retry(
    ///         &self,
    ///         got: Requested<'_, W, StepRetry>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         println!("retrying {} after: {}", got.step_name(), got.cause());
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    fn on_step_retry(
        &self,
        got: Requested<'_, W, StepRetry>,
    ) -> Result<Option<FailWorkflow>, Error>;
}

/// `on step abnormal termination`, called first after an attempt that ended in an abnormal
/// termination, before the step is retried or given up.
///
/// Returning `None` keeps the step's own rule; `FailWorkflow` gives the step up and fails the
/// journey with its reason, without calling `on step failure`. An `Err` aborts the journey with
/// `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{
///     FailWorkflow, OnStepAbnormalTermination, Requested, StepAbnormalTermination,
/// };
/// use itinera::step::Reason;
///
/// struct NeverRetryCrashes;
///
/// impl<W: Send + Sync + 'static> OnStepAbnormalTermination<W> for NeverRetryCrashes {
///     fn on_step_abnormal_termination(
///         &self,
///         _got: Requested<'_, W, StepAbnormalTermination>,
///     ) -> Result<Option<FailWorkflow>, Error> {
///         Ok(Some(FailWorkflow::from(Reason::new("crashed"))))
///     }
/// }
/// ```
pub trait OnStepAbnormalTermination<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, OnStepAbnormalTermination, StepAbnormalTermination};
    ///
    /// fn needs_of<P: OnStepAbnormalTermination<()>>() -> HookNeeds<StepAbnormalTermination> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<StepAbnormalTermination> {
        HookNeeds::new()
    }

    /// Acts on the attempt that ended abnormally.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     FailWorkflow, HookNeeds, OnStepAbnormalTermination, Requested, StepAbnormalTermination,
    /// };
    ///
    /// struct Log;
    ///
    /// impl<W: Send + Sync + 'static> OnStepAbnormalTermination<W> for Log {
    ///     fn needs() -> HookNeeds<StepAbnormalTermination> {
    ///         HookNeeds::new().error()
    ///     }
    ///
    ///     fn on_step_abnormal_termination(
    ///         &self,
    ///         mut got: Requested<'_, W, StepAbnormalTermination>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         println!("{} crashed: {}", got.step_name(), got.error()?);
    ///         Ok(None)
    ///     }
    /// }
    /// ```
    fn on_step_abnormal_termination(
        &self,
        got: Requested<'_, W, StepAbnormalTermination>,
    ) -> Result<Option<FailWorkflow>, Error>;
}

/// `on workflow success`, called once when the journey succeeds, before `journey_succeeded`. It
/// returns no lifecycle. An `Err` aborts the journey with `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{OnWorkflowSuccess, Requested, WorkflowSuccess};
///
/// struct Celebrate;
///
/// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Celebrate {
///     fn on_workflow_success(&self, got: Requested<'_, W, WorkflowSuccess>) -> Result<(), Error> {
///         println!("journey {} succeeded", got.journey_id());
///         Ok(())
///     }
/// }
/// ```
pub trait OnWorkflowSuccess<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, WorkflowSuccess};
    ///
    /// fn needs_of<P: OnWorkflowSuccess<()>>() -> HookNeeds<WorkflowSuccess> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<WorkflowSuccess> {
        HookNeeds::new()
    }

    /// Acts on the journey that succeeded.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// struct Close;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Close {
    ///     fn needs() -> HookNeeds<WorkflowSuccess> {
    ///         HookNeeds::new().contributor()
    ///     }
    ///
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         got.contributor()?.contribute("closed", true);
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn on_workflow_success(&self, got: Requested<'_, W, WorkflowSuccess>) -> Result<(), Error>;
}

/// `on workflow failure`, called once when the journey fails, before `journey_failed`. It
/// returns no lifecycle. An `Err` aborts the journey with `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{OnWorkflowFailure, Requested, WorkflowFailure};
///
/// struct Apologise;
///
/// impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Apologise {
///     fn on_workflow_failure(&self, got: Requested<'_, W, WorkflowFailure>) -> Result<(), Error> {
///         println!("journey {} failed", got.journey_id());
///         Ok(())
///     }
/// }
/// ```
pub trait OnWorkflowFailure<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, OnWorkflowFailure, WorkflowFailure};
    ///
    /// fn needs_of<P: OnWorkflowFailure<()>>() -> HookNeeds<WorkflowFailure> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<WorkflowFailure> {
        HookNeeds::new()
    }

    /// Acts on the journey that failed.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowFailure, Requested, WorkflowFailure};
    ///
    /// struct Alert;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Alert {
    ///     fn needs() -> HookNeeds<WorkflowFailure> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     fn on_workflow_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowFailure>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.warning("the order failed")?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn on_workflow_failure(&self, got: Requested<'_, W, WorkflowFailure>) -> Result<(), Error>;
}
