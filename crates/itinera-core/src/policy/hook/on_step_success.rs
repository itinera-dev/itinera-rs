//! The synchronous `on step success` hook.

use crate::error::Error;
use crate::policy::{HookNeeds, OnSuccess, Requested, StepSuccess};

/// `on step success`, called after each attempt of a step that succeeded, once its
/// contributions are committed. A step policy implements it for the workflows `W` it may be
/// attached to, and its descriptor declares it with
/// [`StepPolicyDescriptor::on_step_success`](crate::policy::StepPolicyDescriptor::on_step_success).
///
/// Returning [`OnSuccess::KeepDefault`] goes on to the next step; `FinishWorkflow` or
/// `FailWorkflow` ends the journey at once. An `Err` aborts the journey with `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
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
///     ) -> Result<OnSuccess> {
///         Ok(if got.step_name() == CHARGE {
///             OnSuccess::FinishWorkflow
///         } else {
///             OnSuccess::KeepDefault
///         })
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
    /// use itinera::error::Result;
    /// use itinera::policy::{OnStepSuccess, OnSuccess, Requested, StepSuccess};
    ///
    /// struct OnlyOnce;
    ///
    /// impl<W: Send + Sync + 'static> OnStepSuccess<W> for OnlyOnce {
    ///     fn on_step_success(
    ///         &self,
    ///         got: Requested<'_, W, StepSuccess>,
    ///     ) -> Result<OnSuccess> {
    ///         if !got.attempt().is_first() {
    ///             return Ok(OnSuccess::fail_workflow("not idempotent"));
    ///         }
    ///         Ok(OnSuccess::KeepDefault)
    ///     }
    /// }
    /// ```
    fn on_step_success(&self, got: Requested<'_, W, StepSuccess>) -> Result<OnSuccess, Error>;
}
