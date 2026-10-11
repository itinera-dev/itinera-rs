//! The synchronous `on workflow success` hook.

use crate::error::Error;
use crate::policy::{HookNeeds, Requested, WorkflowSuccess};

/// `on workflow success`, called once when the journey succeeds, before `journey_succeeded`. It
/// returns no lifecycle. An `Err` aborts the journey with `hook failed`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{OnWorkflowSuccess, Requested, WorkflowSuccess};
///
/// struct Celebrate;
///
/// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Celebrate {
///     fn on_workflow_success(&self, got: Requested<'_, W, WorkflowSuccess>) -> Result<(), Error> {
///         println!("journey {} succeeded", got.journey_id());
///         Ok(())
///     }
/// }
/// ```
pub trait OnWorkflowSuccess<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, WorkflowSuccess};
    ///
    /// fn needs_of<P: OnWorkflowSuccess<()>>() -> HookNeeds<WorkflowSuccess> {
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
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// struct Close;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Close {
    ///     fn needs() -> HookNeeds<WorkflowSuccess> {
    ///         HookNeeds::new().contributor()
    ///     }
    ///
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         got.contributor()?.contribute("closed", true);
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn on_workflow_success(&self, got: Requested<'_, W, WorkflowSuccess>) -> Result<(), Error>;
}
