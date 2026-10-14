//! A reporter of either kind, as an asynchronous dispatcher holds it.

use super::AsyncReporter;
use crate::error::Error;
use crate::event::Event;
use crate::report::Reporter;
use crate::report::sealed::Held;
use crate::value::AnyValue;

/// A reporter of either kind, synchronous or asynchronous, as an asynchronous dispatcher
/// holds it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::report::{BoxedReporter, Reporter};
///
/// struct Silent;
///
/// impl Reporter for Silent {
///     fn report(&mut self, _event: &Event) -> Result<(), Error> {
///         Ok(())
///     }
/// }
///
/// let reporter = BoxedReporter::from_reporter(Silent);
/// ```
#[derive(derive_more::Debug, derive_more::From)]
#[debug("BoxedReporter({kind:?})")]
#[from(Box<dyn Reporter>)]
pub struct BoxedReporter {
    kind: Kind,
}

#[derive(derive_more::Debug, derive_more::From)]
enum Kind {
    #[debug("{:?}", "sync")]
    #[from]
    Sync(Box<dyn Reporter>),
    #[debug("{:?}", "async")]
    Async(Box<dyn DynAsyncReporter>),
}

impl BoxedReporter {
    /// Boxes a synchronous reporter.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::{BoxedReporter, Reporter};
    ///
    /// struct Silent;
    ///
    /// impl Reporter for Silent {
    ///     fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let reporter = BoxedReporter::from_reporter(Silent);
    /// ```
    pub fn from_reporter(reporter: impl Reporter) -> Self {
        Self {
            kind: Kind::Sync(Box::new(reporter)),
        }
    }

    /// Boxes an asynchronous reporter.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::{AsyncReporter, BoxedReporter};
    ///
    /// struct Silent;
    ///
    /// impl AsyncReporter for Silent {
    ///     async fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let reporter = BoxedReporter::from_async_reporter(Silent);
    /// ```
    pub fn from_async_reporter(reporter: impl AsyncReporter) -> Self {
        Self {
            kind: Kind::Async(Box::new(reporter)),
        }
    }

    /// Gives one event to the reporter: called at once if it is synchronous, awaited if it
    /// is asynchronous.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::BoxedReporter;
    ///
    /// async fn deliver(reporter: &mut BoxedReporter, event: &Event) -> Result<(), Error> {
    ///     reporter.report(event).await
    /// }
    /// ```
    pub async fn report(&mut self, event: &Event) -> Result<(), Error> {
        match &mut self.kind {
            Kind::Sync(reporter) => reporter.report(event),
            Kind::Async(reporter) => reporter.report_boxed(event).await,
        }
    }

    /// Gives one entry of the journey's global metadata to the reporter, of either kind.
    ///
    /// # Errors
    ///
    /// The reporter's own error, if it fails on the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::report::BoxedReporter;
    /// use itinera::value::AnyValue;
    ///
    /// fn tell(reporter: &mut BoxedReporter, user: &str) -> Result<(), Error> {
    ///     reporter.set_global_metadata("user_id", &AnyValue::new(user.to_string()))
    /// }
    /// ```
    pub fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        match &mut self.kind {
            Kind::Sync(reporter) => reporter.set_global_metadata(key, value),
            Kind::Async(reporter) => reporter.set_global_metadata(key, value),
        }
    }
}

#[async_trait::async_trait]
trait DynAsyncReporter: Send {
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error>;

    async fn report_boxed(&mut self, event: &Event) -> Result<(), Error>;
}

#[async_trait::async_trait]
impl<R: AsyncReporter> DynAsyncReporter for R {
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        AsyncReporter::set_global_metadata(self, key, value)
    }

    async fn report_boxed(&mut self, event: &Event) -> Result<(), Error> {
        AsyncReporter::report(self, event).await
    }
}

impl Held for BoxedReporter {
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        BoxedReporter::set_global_metadata(self, key, value)
    }
}
