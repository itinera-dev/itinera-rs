//! The reporter an asynchronous workflow's policy hook takes from what it received.

use super::AsyncHookReporter;
use crate::error::Error;
use crate::mode::Asynchronous;
use crate::policy::{PolicyHookKind, Requested};

impl<'a, W, H: PolicyHookKind> Requested<'a, W, H, Asynchronous> {
    /// Takes the reporter the hook declared.
    ///
    /// # Errors
    ///
    /// When the hook did not declare a reporter, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{AsyncOnWorkflowFailure, HookNeeds, Requested, WorkflowFailure};
    ///
    /// struct Alert;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnWorkflowFailure<W> for Alert {
    ///     fn needs() -> HookNeeds<WorkflowFailure> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     async fn on_workflow_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowFailure, Asynchronous>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.error("the order failed").await?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn reporter(&mut self) -> Result<AsyncHookReporter<'a>, Error> {
        self.take_reporting().map(AsyncHookReporter::from)
    }
}
