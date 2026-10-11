//! The asynchronous `on step success` hook.

use std::future::Future;

use crate::error::Error;
use crate::mode::Asynchronous;
use crate::policy::{HookNeeds, OnSuccess, Requested, StepSuccess};

/// `on step success` of an asynchronous workflow's policy. Everything
/// [`OnStepSuccess`](crate::policy::OnStepSuccess) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{AsyncOnStepSuccess, OnSuccess, Requested, StepSuccess};
/// use itinera::step::StepName;
///
/// const CHARGE: StepName = StepName::new("charge");
///
/// struct StopAfterCharge;
///
/// impl<W: Send + Sync + 'static> AsyncOnStepSuccess<W> for StopAfterCharge {
///     async fn on_step_success(
///         &self,
///         got: Requested<'_, W, StepSuccess, Asynchronous>,
///     ) -> Result<Option<OnSuccess>, Error> {
///         Ok((got.step_name() == CHARGE).then_some(OnSuccess::FinishWorkflow))
///     }
/// }
/// ```
pub trait AsyncOnStepSuccess<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnStepSuccess, HookNeeds, StepSuccess};
    ///
    /// fn needs_of<P: AsyncOnStepSuccess<()>>() -> HookNeeds<StepSuccess> {
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
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{AsyncOnStepSuccess, OnSuccess, Requested, StepSuccess};
    ///
    /// async fn decide<P: AsyncOnStepSuccess<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), StepSuccess, Asynchronous>,
    /// ) -> Result<Option<OnSuccess>, Error> {
    ///     policy.on_step_success(got).await
    /// }
    /// ```
    fn on_step_success(
        &self,
        got: Requested<'_, W, StepSuccess, Asynchronous>,
    ) -> impl Future<Output = Result<Option<OnSuccess>, Error>> + Send;
}
