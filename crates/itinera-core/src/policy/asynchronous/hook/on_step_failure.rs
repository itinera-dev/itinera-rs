//! The asynchronous `on step failure` hook.

use std::future::Future;

use crate::error::Error;
use crate::mode::Asynchronous;
use crate::policy::{FailWorkflow, HookNeeds, Requested, StepFailure};

/// `on step failure` of an asynchronous workflow's policy. Everything
/// [`OnStepFailure`](crate::policy::OnStepFailure) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{AsyncOnStepFailure, FailWorkflow, Requested, StepFailure};
///
/// struct Alarm;
///
/// impl<W: Send + Sync + 'static> AsyncOnStepFailure<W> for Alarm {
///     async fn on_step_failure(
///         &self,
///         got: Requested<'_, W, StepFailure, Asynchronous>,
///     ) -> Result<Option<FailWorkflow>, Error> {
///         println!("{} was given up", got.step_name());
///         Ok(None)
///     }
/// }
/// ```
pub trait AsyncOnStepFailure<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnStepFailure, HookNeeds, StepFailure};
    ///
    /// fn needs_of<P: AsyncOnStepFailure<()>>() -> HookNeeds<StepFailure> {
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
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{AsyncOnStepFailure, FailWorkflow, Requested, StepFailure};
    ///
    /// async fn decide<P: AsyncOnStepFailure<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), StepFailure, Asynchronous>,
    /// ) -> Result<Option<FailWorkflow>, Error> {
    ///     policy.on_step_failure(got).await
    /// }
    /// ```
    fn on_step_failure(
        &self,
        got: Requested<'_, W, StepFailure, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send;
}
