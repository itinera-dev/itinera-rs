//! The asynchronous `on workflow failure` hook.

use std::future::Future;

use crate::error::Error;
use crate::mode::Asynchronous;
use crate::policy::{HookNeeds, Requested, WorkflowFailure};

/// `on workflow failure` of an asynchronous workflow's policy. Everything
/// [`OnWorkflowFailure`](crate::policy::OnWorkflowFailure) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{AsyncOnWorkflowFailure, Requested, WorkflowFailure};
///
/// struct Apologise;
///
/// impl<W: Send + Sync + 'static> AsyncOnWorkflowFailure<W> for Apologise {
///     async fn on_workflow_failure(
///         &self,
///         got: Requested<'_, W, WorkflowFailure, Asynchronous>,
///     ) -> Result<(), Error> {
///         println!("journey {} failed", got.journey_id());
///         Ok(())
///     }
/// }
/// ```
pub trait AsyncOnWorkflowFailure<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnWorkflowFailure, HookNeeds, WorkflowFailure};
    ///
    /// fn needs_of<P: AsyncOnWorkflowFailure<()>>() -> HookNeeds<WorkflowFailure> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<WorkflowFailure> {
        HookNeeds::new()
    }

    /// Acts on the journey that failed.
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
    /// use itinera::policy::{AsyncOnWorkflowFailure, Requested, WorkflowFailure};
    ///
    /// async fn close<P: AsyncOnWorkflowFailure<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), WorkflowFailure, Asynchronous>,
    /// ) -> Result<(), Error> {
    ///     policy.on_workflow_failure(got).await
    /// }
    /// ```
    fn on_workflow_failure(
        &self,
        got: Requested<'_, W, WorkflowFailure, Asynchronous>,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}
