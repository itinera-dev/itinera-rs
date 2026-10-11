use std::future::Future;
use std::pin::Pin;

use super::default_dispatcher::Delivery;
use super::{DefaultDispatcher, DefaultDispatcherFactory, Reporter};
use crate::error::Error;
use crate::event::Event;
use crate::journey::{DataBag, JourneyId};

/// A [`Reporter`] whose `report` is asynchronous. A workflow with one is asynchronous.
///
/// What [`Reporter`] says about failing applies to it too.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::report::AsyncReporter;
///
/// struct Forward;
///
/// impl AsyncReporter for Forward {
///     async fn report(&mut self, _event: &Event) -> Result<(), Error> {
///         Ok(())
///     }
/// }
/// ```
pub trait AsyncReporter: Send + 'static {
    /// Receives one event.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::AsyncReporter;
    ///
    /// struct Silent;
    ///
    /// impl AsyncReporter for Silent {
    ///     async fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn report(&mut self, event: &Event) -> impl Future<Output = Result<(), Error>> + Send;
}

/// An [`AsyncReporter`] that a workflow lists, which every instance of the workflow makes for
/// itself. Only an asynchronous workflow lists one.
///
/// What [`WorkflowReporter`](super::WorkflowReporter) says about making reporters applies to it
/// too; making it is synchronous.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::journey::{DataBag, JourneyId};
/// use itinera::report::{AsyncReporter, AsyncWorkflowReporter};
///
/// struct Orders;
///
/// struct Forward;
///
/// impl AsyncReporter for Forward {
///     async fn report(&mut self, _event: &Event) -> Result<(), Error> {
///         Ok(())
///     }
/// }
///
/// impl AsyncWorkflowReporter<Orders> for Forward {
///     fn init(_: &Orders, _: &JourneyId, _: &DataBag) -> Result<Self, Error> {
///         Ok(Forward)
///     }
/// }
/// ```
pub trait AsyncWorkflowReporter<W>: AsyncReporter + Sized {
    /// Makes the reporter for one journey, from the workflow's own value, the journey ID and the
    /// initial data. Failing makes creating the instance fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::journey::{DataBag, JourneyId};
    /// use itinera::report::{AsyncReporter, AsyncWorkflowReporter};
    ///
    /// struct Orders;
    ///
    /// struct Forward {
    ///     journey: JourneyId,
    /// }
    ///
    /// impl AsyncReporter for Forward {
    ///     async fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// impl AsyncWorkflowReporter<Orders> for Forward {
    ///     fn init(_: &Orders, journey_id: &JourneyId, _: &DataBag) -> Result<Self, Error> {
    ///         Ok(Forward {
    ///             journey: journey_id.clone(),
    ///         })
    ///     }
    /// }
    /// ```
    fn init(workflow: &W, journey_id: &JourneyId, data: &DataBag) -> Result<Self, Error>;
}

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
#[derive(derive_more::Debug)]
#[debug("BoxedReporter({kind:?})")]
pub struct BoxedReporter {
    kind: Kind,
}

#[derive(derive_more::Debug)]
enum Kind {
    #[debug("{:?}", "sync")]
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

impl From<Box<dyn Reporter>> for BoxedReporter {
    fn from(reporter: Box<dyn Reporter>) -> Self {
        Self {
            kind: Kind::Sync(reporter),
        }
    }
}

type BoxedFuture<'a> = Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;

trait DynAsyncReporter: Send {
    fn report_boxed<'a>(&'a mut self, event: &'a Event) -> BoxedFuture<'a>;
}

impl<R: AsyncReporter> DynAsyncReporter for R {
    fn report_boxed<'a>(&'a mut self, event: &'a Event) -> BoxedFuture<'a> {
        Box::pin(AsyncReporter::report(self, event))
    }
}

/// A [`Dispatcher`](super::Dispatcher) whose operations are asynchronous, and which holds
/// reporters of either kind. The asynchronous executor uses one.
///
/// What [`Dispatcher`](super::Dispatcher) says about failing applies to it too.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::report::{AsyncDispatcher, BoxedReporter};
///
/// struct OnlyMine {
///     reporter: BoxedReporter,
/// }
///
/// impl AsyncDispatcher for OnlyMine {
///     async fn add(&mut self, _reporter: BoxedReporter) -> Result<(), Error> {
///         Ok(())
///     }
///
///     async fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
///         self.reporter.report(event).await
///     }
/// }
/// ```
pub trait AsyncDispatcher: Send + 'static {
    /// Adds a reporter for the journey.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::report::{AsyncDispatcher, BoxedReporter};
    ///
    /// async fn add_all(
    ///     dispatcher: &mut impl AsyncDispatcher,
    ///     reporters: Vec<BoxedReporter>,
    /// ) -> Result<(), Error> {
    ///     for reporter in reporters {
    ///         dispatcher.add(reporter).await?;
    ///     }
    ///     Ok(())
    /// }
    /// ```
    fn add(&mut self, reporter: BoxedReporter) -> impl Future<Output = Result<(), Error>> + Send;

    /// Delivers one event.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::AsyncDispatcher;
    ///
    /// async fn deliver(dispatcher: &mut impl AsyncDispatcher, event: &Event) -> Result<(), Error> {
    ///     dispatcher.dispatch(event).await
    /// }
    /// ```
    fn dispatch(&mut self, event: &Event) -> impl Future<Output = Result<(), Error>> + Send;
}

/// Creates a new [`AsyncDispatcher`] for every journey run by the asynchronous executor.
///
/// Failing refuses the journey before it starts.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::report::{AsyncDispatcherFactory, BoxedReporter, DefaultDispatcher};
///
/// struct Fresh;
///
/// impl AsyncDispatcherFactory for Fresh {
///     type Dispatcher = DefaultDispatcher<BoxedReporter>;
///
///     async fn create(&mut self) -> Result<Self::Dispatcher, Error> {
///         Ok(DefaultDispatcher::default())
///     }
/// }
/// ```
pub trait AsyncDispatcherFactory: Send + 'static {
    /// The dispatchers it creates.
    type Dispatcher: AsyncDispatcher;

    /// Creates the dispatcher for one journey.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::report::AsyncDispatcherFactory;
    ///
    /// async fn fresh<F: AsyncDispatcherFactory>(factory: &mut F) -> Result<F::Dispatcher, Error> {
    ///     factory.create().await
    /// }
    /// ```
    fn create(&mut self) -> impl Future<Output = Result<Self::Dispatcher, Error>> + Send;
}

impl AsyncDispatcher for DefaultDispatcher<BoxedReporter> {
    async fn add(&mut self, reporter: BoxedReporter) -> Result<(), Error> {
        self.reporters.push(reporter);
        Ok(())
    }

    async fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
        let mut delivery = Delivery::of(event);
        for reporter in &mut self.reporters {
            if delivery.failed(reporter.report(event).await) {
                break;
            }
        }
        delivery.finish()
    }
}

impl AsyncDispatcherFactory for DefaultDispatcherFactory {
    type Dispatcher = DefaultDispatcher<BoxedReporter>;

    async fn create(&mut self) -> Result<Self::Dispatcher, Error> {
        Ok(DefaultDispatcher::default())
    }
}
