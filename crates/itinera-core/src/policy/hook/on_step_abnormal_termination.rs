//! The synchronous `on step abnormal termination` hook.

use crate::error::Error;
use crate::policy::{FailWorkflow, HookNeeds, Requested, StepAbnormalTermination};

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
