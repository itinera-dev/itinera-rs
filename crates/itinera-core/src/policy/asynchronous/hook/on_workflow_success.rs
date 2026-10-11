//! The asynchronous `on workflow success` hook.

use std::future::Future;

use crate::error::Error;
use crate::mode::Asynchronous;
use crate::policy::{HookNeeds, Requested, WorkflowSuccess};

/// `on workflow success` of an asynchronous workflow's policy. Everything
/// [`OnWorkflowSuccess`](crate::policy::OnWorkflowSuccess) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{AsyncOnWorkflowSuccess, Requested, WorkflowSuccess};
///
/// struct Celebrate;
///
/// impl<W: Send + Sync + 'static> AsyncOnWorkflowSuccess<W> for Celebrate {
///     async fn on_workflow_success(
///         &self,
///         got: Requested<'_, W, WorkflowSuccess, Asynchronous>,
///     ) -> Result<(), Error> {
///         println!("journey {} succeeded", got.journey_id());
///         Ok(())
///     }
/// }
/// ```
pub trait AsyncOnWorkflowSuccess<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnWorkflowSuccess, HookNeeds, WorkflowSuccess};
    ///
    /// fn needs_of<P: AsyncOnWorkflowSuccess<()>>() -> HookNeeds<WorkflowSuccess> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<WorkflowSuccess> {
        HookNeeds::new()
    }

    /// Acts on the journey that succeeded.
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
    /// use itinera::policy::{AsyncOnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// async fn close<P: AsyncOnWorkflowSuccess<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), WorkflowSuccess, Asynchronous>,
    /// ) -> Result<(), Error> {
    ///     policy.on_workflow_success(got).await
    /// }
    /// ```
    fn on_workflow_success(
        &self,
        got: Requested<'_, W, WorkflowSuccess, Asynchronous>,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
