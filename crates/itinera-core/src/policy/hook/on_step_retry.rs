//! The synchronous `on step retry` hook.

use crate::error::Error;
use crate::policy::{HookNeeds, OnFailure, Requested, StepRetry};

/// `on step retry`, called when a step will be attempted again, before `step_retrying`.
///
/// Returning [`OnFailure::KeepDefault`] lets the step be attempted again; `FailWorkflow` gives the
/// step up and fails the journey with its reason, without calling `on step failure`. An `Err`
/// aborts the journey with `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::policy::{OnFailure, OnStepRetry, Requested, StepRetry};
///
/// struct AtMostTwice;
///
/// impl<W: Send + Sync + 'static> OnStepRetry<W> for AtMostTwice {
///     fn on_step_retry(
///         &self,
///         got: Requested<'_, W, StepRetry>,
///     ) -> Result<OnFailure> {
///         Ok(if !got.attempt().is_first() {
///             OnFailure::fail_workflow("too many tries")
///         } else {
///             OnFailure::KeepDefault
///         })
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
    /// use itinera::error::Result;
    /// use itinera::policy::{OnFailure, OnStepRetry, Requested, StepRetry};
    ///
    /// struct Log;
    ///
    /// impl<W: Send + Sync + 'static> OnStepRetry<W> for Log {
    ///     fn on_step_retry(
    ///         &self,
    ///         got: Requested<'_, W, StepRetry>,
    ///     ) -> Result<OnFailure> {
    ///         println!("retrying {} after: {}", got.step_name(), got.cause());
    ///         Ok(OnFailure::KeepDefault)
    ///     }
    /// }
    /// ```
    fn on_step_retry(&self, got: Requested<'_, W, StepRetry>) -> Result<OnFailure, Error>;
}
