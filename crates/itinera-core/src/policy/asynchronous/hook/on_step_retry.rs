//! The asynchronous `on step retry` hook.

use std::future::Future;

use crate::error::Error;
use crate::mode::Asynchronous;
use crate::policy::{FailWorkflow, HookNeeds, Requested, StepRetry};

/// `on step retry` of an asynchronous workflow's policy. Everything
/// [`OnStepRetry`](crate::policy::OnStepRetry) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{AsyncOnStepRetry, FailWorkflow, Requested, StepRetry};
///
/// struct Patience;
///
/// impl<W: Send + Sync + 'static> AsyncOnStepRetry<W> for Patience {
///     async fn on_step_retry(
///         &self,
///         got: Requested<'_, W, StepRetry, Asynchronous>,
///     ) -> Result<Option<FailWorkflow>, Error> {
///         println!("{} is attempted again", got.step_name());
///         Ok(None)
///     }
/// }
/// ```
pub trait AsyncOnStepRetry<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnStepRetry, HookNeeds, StepRetry};
    ///
    /// fn needs_of<P: AsyncOnStepRetry<()>>() -> HookNeeds<StepRetry> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<StepRetry> {
        HookNeeds::new()
    }

    /// Acts on the step before it is attempted again.
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
    /// use itinera::policy::{AsyncOnStepRetry, FailWorkflow, Requested, StepRetry};
    ///
    /// async fn decide<P: AsyncOnStepRetry<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), StepRetry, Asynchronous>,
    /// ) -> Result<Option<FailWorkflow>, Error> {
    ///     policy.on_step_retry(got).await
    /// }
    /// ```
    fn on_step_retry(
        &self,
        got: Requested<'_, W, StepRetry, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send;
}
