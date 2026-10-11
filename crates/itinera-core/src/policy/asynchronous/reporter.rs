//! The reporter of an asynchronous workflow's policy hook.

use std::future::Future;

use crate::error::Interrupted;
use crate::step::{Level, Reporting};
use crate::value::{AnyValue, Value};

/// An asynchronous hook's means of emitting its own events, `journey_info`, `journey_warning`
/// and `journey_error`. Everything [`HookReporter`](crate::policy::HookReporter) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Interrupted;
/// use itinera::policy::AsyncHookReporter;
///
/// async fn closing(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
///     reporter.info("closing").await
/// }
/// ```
#[derive(Debug, derive_more::From)]
pub struct AsyncHookReporter<'a> {
    reporting: Reporting<'a>,
}

impl AsyncHookReporter<'_> {
    /// Emits `journey_info` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn closing(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.info("closing").await
    /// }
    /// ```
    pub fn info(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Info, message.into(), None)
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
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn closing(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.info_with("closing", 42_i64).await
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

    /// Emits `journey_warning` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn late(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.warning("the order is late").await
    /// }
    /// ```
    pub fn warning(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Warning, message.into(), None)
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
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn late(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.warning_with("the order is late", 42_i64).await
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

    /// Emits `journey_error` with a message. It changes nothing in the journey.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn unsent(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.error("the receipt was not sent").await
    /// }
    /// ```
    pub fn error(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Error, message.into(), None)
    }

    /// Emits `journey_error` with a message and data, a value. It changes nothing in the
    /// journey.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn unsent(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.error_with("the receipt was not sent", 42_i64).await
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
