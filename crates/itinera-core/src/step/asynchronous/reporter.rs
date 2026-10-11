//! An asynchronous step's reporter, for its own events.

use std::future::Future;

use crate::error::Interrupted;
use crate::step::{Level, Reporting};
use crate::value::{AnyValue, Value};

/// An asynchronous step's means of emitting its own events, `step_info`, `step_warning` and
/// `step_error`, each stamped with the step and its attempt.
///
/// Everything [`StepReporter`](crate::step::StepReporter) says applies to it, except that each
/// emit call is awaited: the step waits there until every reporter has the event.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::step::{
///     AsyncStep, AsyncStepFactory, AsyncStepReporter, Outcome, Resolved, StepNeeds,
/// };
///
/// struct Charge<'a> {
///     reporter: AsyncStepReporter<'a>,
/// }
///
/// impl AsyncStep for Charge<'_> {
///     async fn run(mut self) -> Result<Outcome, Error> {
///         self.reporter.info_with("charging", 42_i64).await?;
///         Ok(Outcome::success())
///     }
/// }
///
/// struct ChargeFactory;
///
/// impl AsyncStepFactory for ChargeFactory {
///     type Step<'a> = Charge<'a>;
///
///     fn needs(&self) -> StepNeeds {
///         StepNeeds::new().reporter()
///     }
///
///     fn build<'a>(&'a self, got: &mut Resolved<'a, Asynchronous>) -> Result<Charge<'a>, Error> {
///         Ok(Charge {
///             reporter: got.reporter()?,
///         })
///     }
/// }
/// ```
#[derive(Debug, derive_more::From)]
pub struct AsyncStepReporter<'a> {
    reporting: Reporting<'a>,
}

impl AsyncStepReporter<'_> {
    /// Emits `step_info` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn starting(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.info("starting").await
    /// }
    /// ```
    pub fn info(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Info, message.into(), None)
    }

    /// Emits `step_info` with a message and data.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn starting(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.info_with("starting", 42_i64).await
    /// }
    /// ```
    pub fn info_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting
            .emit(Level::Info, message.into(), Some(AnyValue::new(data)))
    }

    /// Emits `step_warning` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn slow(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.warning("the gateway is slow").await
    /// }
    /// ```
    pub fn warning(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Warning, message.into(), None)
    }

    /// Emits `step_warning` with a message and data.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn slow(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.warning_with("the gateway is slow", 42_i64).await
    /// }
    /// ```
    pub fn warning_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting
            .emit(Level::Warning, message.into(), Some(AnyValue::new(data)))
    }

    /// Emits `step_error` with a message. It does not change the step's outcome.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn no_receipt(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.error("the receipt was not printed").await
    /// }
    /// ```
    pub fn error(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Error, message.into(), None)
    }

    /// Emits `step_error` with a message and data. It does not change the step's outcome.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::AsyncStepReporter;
    ///
    /// async fn no_receipt(reporter: &mut AsyncStepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.error_with("the receipt was not printed", 42_i64).await
    /// }
    /// ```
    pub fn error_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting
            .emit(Level::Error, message.into(), Some(AnyValue::new(data)))
    }
}
