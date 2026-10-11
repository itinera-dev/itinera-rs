//! The synchronous `on step failure` hook.

use crate::error::Error;
use crate::policy::{FailWorkflow, HookNeeds, Requested, StepFailure};

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
