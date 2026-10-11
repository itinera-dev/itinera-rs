//! The synchronous `on step retry` hook.

use crate::error::Error;
use crate::policy::{FailWorkflow, HookNeeds, Requested, StepRetry};

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
