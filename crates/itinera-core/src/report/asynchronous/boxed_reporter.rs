//! A reporter of either kind, as an asynchronous dispatcher holds it.

use std::future::Future;
use std::pin::Pin;

use super::AsyncReporter;
use crate::error::Error;
use crate::event::Event;
use crate::journey::JourneyId;
use crate::report::Reporter;
use crate::report::sealed::Held;

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
}

type BoxedFuture<'a> = Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;

trait DynAsyncReporter: Send {
    fn set_journey_id(&mut self, journey_id: &JourneyId) -> Result<(), Error>;

    fn report_boxed<'a>(&'a mut self, event: &'a Event) -> BoxedFuture<'a>;
}

impl<R: AsyncReporter> DynAsyncReporter for R {
    fn set_journey_id(&mut self, journey_id: &JourneyId) -> Result<(), Error> {
        AsyncReporter::set_journey_id(self, journey_id)
    }

    fn report_boxed<'a>(&'a mut self, event: &'a Event) -> BoxedFuture<'a> {
        Box::pin(AsyncReporter::report(self, event))
    }
}

impl Held for BoxedReporter {
    fn set_journey_id(&mut self, journey_id: &JourneyId) -> Result<(), Error> {
        match &mut self.kind {
            Kind::Sync(reporter) => reporter.set_journey_id(journey_id),
            Kind::Async(reporter) => reporter.set_journey_id(journey_id),
        }
    }
}
