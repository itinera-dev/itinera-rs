use std::future::Future;

use crate::error::Error;
use crate::event::Event;
use crate::journey::{DataBag, JourneyId};

mod boxed_reporter;
mod dispatcher;

pub use boxed_reporter::BoxedReporter;
pub use dispatcher::{AsyncDispatcher, AsyncDispatcherFactory};

/// A [`Reporter`] whose `report` is asynchronous. A workflow with one is asynchronous.
///
/// What [`Reporter`] says about failing applies to it too.
///
/// [`Reporter`]: super::Reporter
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
