//! The synchronous `on workflow failure` hook.

use crate::error::Error;
use crate::policy::{HookNeeds, Requested, WorkflowFailure};

/// `on workflow failure`, called once when the journey fails, before `journey_failed`. It
/// returns no lifecycle. An `Err` aborts the journey with `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{OnWorkflowFailure, Requested, WorkflowFailure};
///
/// struct Apologise;
///
/// impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Apologise {
///     fn on_workflow_failure(&self, got: Requested<'_, W, WorkflowFailure>) -> Result<(), Error> {
///         println!("journey {} failed", got.journey_id());
///         Ok(())
///     }
/// }
/// ```
pub trait OnWorkflowFailure<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, OnWorkflowFailure, WorkflowFailure};
    ///
    /// fn needs_of<P: OnWorkflowFailure<()>>() -> HookNeeds<WorkflowFailure> {
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
    /// use itinera::policy::{HookNeeds, OnWorkflowFailure, Requested, WorkflowFailure};
    ///
    /// struct Alert;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Alert {
    ///     fn needs() -> HookNeeds<WorkflowFailure> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     fn on_workflow_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowFailure>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.warning("the order failed")?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn on_workflow_failure(&self, got: Requested<'_, W, WorkflowFailure>) -> Result<(), Error>;
}
