//! The asynchronous `on step abnormal termination` hook.

use std::future::Future;

use crate::error::Error;
use crate::mode::Asynchronous;
use crate::policy::{FailWorkflow, HookNeeds, Requested, StepAbnormalTermination};

/// `on step abnormal termination` of an asynchronous workflow's policy. Everything
/// [`OnStepAbnormalTermination`](crate::policy::OnStepAbnormalTermination) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{
///     AsyncOnStepAbnormalTermination, FailWorkflow, Requested, StepAbnormalTermination,
/// };
///
/// struct Crashes;
///
/// impl<W: Send + Sync + 'static> AsyncOnStepAbnormalTermination<W> for Crashes {
///     async fn on_step_abnormal_termination(
///         &self,
///         got: Requested<'_, W, StepAbnormalTermination, Asynchronous>,
///     ) -> Result<Option<FailWorkflow>, Error> {
///         println!("{} crashed", got.step_name());
///         Ok(None)
///     }
/// }
/// ```
pub trait AsyncOnStepAbnormalTermination<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnStepAbnormalTermination, HookNeeds, StepAbnormalTermination};
    ///
    /// fn needs_of<P: AsyncOnStepAbnormalTermination<()>>() -> HookNeeds<StepAbnormalTermination> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<StepAbnormalTermination> {
        HookNeeds::new()
    }

    /// Acts on the attempt that ended in an abnormal termination.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnStepAbnormalTermination, FailWorkflow, Requested, StepAbnormalTermination,
    /// };
    ///
    /// async fn decide<P: AsyncOnStepAbnormalTermination<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), StepAbnormalTermination, Asynchronous>,
    /// ) -> Result<Option<FailWorkflow>, Error> {
    ///     policy.on_step_abnormal_termination(got).await
    /// }
    /// ```
    fn on_step_abnormal_termination(
        &self,
        got: Requested<'_, W, StepAbnormalTermination, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send;
}
