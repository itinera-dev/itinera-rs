//! The synchronous `on step abnormal termination` hook.

use crate::error::Error;
use crate::policy::{HookNeeds, OnFailure, Requested, StepAbnormalTermination};

/// `on step abnormal termination`, called first after an attempt that ended in an abnormal
/// termination, before the step is retried or given up.
///
/// Returning [`OnFailure::KeepDefault`] keeps the step's own rule; `FailWorkflow` gives the step up
/// and fails the journey with its reason, without calling `on step failure`. An `Err` aborts the
/// journey with `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::policy::{OnFailure, OnStepAbnormalTermination, Requested, StepAbnormalTermination};
///
/// struct NeverRetryCrashes;
///
/// impl<W: Send + Sync + 'static> OnStepAbnormalTermination<W> for NeverRetryCrashes {
///     fn on_step_abnormal_termination(
///         &self,
///         _got: Requested<'_, W, StepAbnormalTermination>,
///     ) -> Result<OnFailure> {
///         Ok(OnFailure::fail_workflow("crashed"))
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
    /// use itinera::error::Result;
    /// use itinera::policy::{
    ///     HookNeeds, OnFailure, OnStepAbnormalTermination, Requested, StepAbnormalTermination,
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
    ///     ) -> Result<OnFailure> {
    ///         println!("{} crashed: {}", got.step_name(), got.error()?);
    ///         Ok(OnFailure::KeepDefault)
    ///     }
    /// }
    /// ```
    fn on_step_abnormal_termination(
        &self,
        got: Requested<'_, W, StepAbnormalTermination>,
    ) -> Result<OnFailure, Error>;
}
