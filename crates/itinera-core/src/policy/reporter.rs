use crate::engine::finish;
use crate::error::Interrupted;
use crate::step::{Level, Reporting};
use crate::value::{AnyValue, Value};

/// A hook's means of emitting its own events, `journey_info`, `journey_warning` and
/// `journey_error`, each stamped with the policy and the hook, and with the step and its attempt
/// for a step hook.
///
/// It is made for one call of the hook and borrows it, so it cannot outlive it. An event is
/// delivered to every reporter before the call returns. If a reporter fails meanwhile, the
/// journey is aborted with `reporter failed`, and the call returns [`Interrupted`], which the
/// hook propagates with `?`: the abort stands whatever the hook does afterwards. Data is a value,
/// captured when the event is emitted.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
///
/// struct Audit;
///
/// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Audit {
///     fn needs() -> HookNeeds<WorkflowSuccess> {
///         HookNeeds::new().reporter()
///     }
///
///     fn on_workflow_success(
///         &self,
///         mut got: Requested<'_, W, WorkflowSuccess>,
///     ) -> Result<(), Error> {
///         got.reporter()?.info_with("closing", 42_i64)?;
///         Ok(())
///     }
/// }
/// ```
#[derive(Debug, derive_more::From)]
pub struct HookReporter<'a> {
    reporting: Reporting<'a>,
}

impl HookReporter<'_> {
    /// Emits `journey_info` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// struct Audit;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Audit {
    ///     fn needs() -> HookNeeds<WorkflowSuccess> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.info("closing")?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn info(&mut self, message: impl Into<String>) -> Result<(), Interrupted> {
        finish(self.reporting.emit(Level::Info, message.into(), None))
    }

    /// Emits `journey_info` with a message and data, a value.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// struct Audit;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Audit {
    ///     fn needs() -> HookNeeds<WorkflowSuccess> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.info_with("closing", 42_i64)?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn info_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> Result<(), Interrupted> {
        finish(
            self.reporting
                .emit(Level::Info, message.into(), Some(AnyValue::new(data))),
        )
    }

    /// Emits `journey_warning` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// struct Audit;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Audit {
    ///     fn needs() -> HookNeeds<WorkflowSuccess> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.warning("closing")?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn warning(&mut self, message: impl Into<String>) -> Result<(), Interrupted> {
        finish(self.reporting.emit(Level::Warning, message.into(), None))
    }

    /// Emits `journey_warning` with a message and data, a value.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// struct Audit;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Audit {
    ///     fn needs() -> HookNeeds<WorkflowSuccess> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.warning_with("closing", 42_i64)?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn warning_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> Result<(), Interrupted> {
        finish(
            self.reporting
                .emit(Level::Warning, message.into(), Some(AnyValue::new(data))),
        )
    }

    /// Emits `journey_error` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// struct Audit;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Audit {
    ///     fn needs() -> HookNeeds<WorkflowSuccess> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.error("closing")?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn error(&mut self, message: impl Into<String>) -> Result<(), Interrupted> {
        finish(self.reporting.emit(Level::Error, message.into(), None))
    }

    /// Emits `journey_error` with a message and data, a value.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, OnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// struct Audit;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Audit {
    ///     fn needs() -> HookNeeds<WorkflowSuccess> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.error_with("closing", 42_i64)?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn error_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> Result<(), Interrupted> {
        finish(
            self.reporting
                .emit(Level::Error, message.into(), Some(AnyValue::new(data))),
        )
    }
}
