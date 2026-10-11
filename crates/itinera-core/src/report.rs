//! Reporters, which receive the events of a journey, and dispatchers, which deliver them.

use crate::error::Error;
use crate::event::Event;
use crate::journey::{DataBag, JourneyId};

#[cfg(feature = "async")]
mod asynchronous;
mod default_dispatcher;
mod dispatcher;
mod sealed;

#[cfg(feature = "async")]
pub use asynchronous::{
    AsyncDispatcher, AsyncDispatcherFactory, AsyncReporter, AsyncWorkflowReporter, BoxedReporter,
};
pub use default_dispatcher::{DefaultDispatcher, DefaultDispatcherFactory};
pub use dispatcher::{Dispatcher, DispatcherFactory};

/// Receives the events of a journey, and decides on its own what to do with each: filter it,
/// map it, forward it or ignore it.
///
/// A workflow lists its reporters, and each journey gets its own.
///
/// A reporter should never fail. One that cannot report an event, for example because the
/// service it forwards to is unavailable, should handle that itself (log it, drop the event,
/// keep it to retry later) and return `Ok`. An `Err` aborts the journey with `reporter failed`,
/// and the event it failed on never reaches the reporters after it, so the order of reporters
/// matters. `journey_aborted` then reaches every reporter except the one that failed. An `Err`
/// while `journey_aborted` itself is reported is ignored.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::report::Reporter;
///
/// #[derive(Default)]
/// struct Kinds {
///     kinds: Vec<&'static str>,
/// }
///
/// impl Reporter for Kinds {
///     fn report(&mut self, event: &Event) -> Result<(), Error> {
///         self.kinds.push(event.kind());
///         Ok(())
///     }
/// }
///
/// let reporter: Box<dyn Reporter> = Box::new(Kinds::default());
/// ```
pub trait Reporter: Send + 'static {
    /// Receives one event.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::Reporter;
    ///
    /// struct Silent;
    ///
    /// impl Reporter for Silent {
    ///     fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn report(&mut self, event: &Event) -> Result<(), Error>;
}

/// A [`Reporter`] that a workflow lists, which every instance of the workflow makes for itself.
///
/// The workflow lists its reporters by type. Creating an instance makes each of them with
/// [`init`](WorkflowReporter::init), after the journey ID, so a reporter can be made for that
/// journey alone.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::journey::{DataBag, JourneyId};
/// use itinera::report::{Reporter, WorkflowReporter};
///
/// struct Orders;
///
/// struct Audit {
///     journey: JourneyId,
/// }
///
/// impl Reporter for Audit {
///     fn report(&mut self, event: &Event) -> Result<(), Error> {
///         eprintln!("{} {}", self.journey, event.kind());
///         Ok(())
///     }
/// }
///
/// impl WorkflowReporter<Orders> for Audit {
///     fn init(_: &Orders, journey_id: &JourneyId, _: &DataBag) -> Result<Self, Error> {
///         Ok(Audit {
///             journey: journey_id.clone(),
///         })
///     }
/// }
/// ```
pub trait WorkflowReporter<W>: Reporter + Sized {
    /// Makes the reporter for one journey, from the workflow's own value, the journey ID and the
    /// initial data. Failing makes creating the instance fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::journey::{DataBag, JourneyId};
    /// use itinera::report::{Reporter, WorkflowReporter};
    ///
    /// struct Orders {
    ///     audited: bool,
    /// }
    ///
    /// struct Audit;
    ///
    /// impl Reporter for Audit {
    ///     fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// impl WorkflowReporter<Orders> for Audit {
    ///     fn init(orders: &Orders, _: &JourneyId, _: &DataBag) -> Result<Self, Error> {
    ///         if orders.audited {
    ///             Ok(Audit)
    ///         } else {
    ///             Err(Error::msg("auditing is off"))
    ///         }
    ///     }
    /// }
    /// ```
    fn init(workflow: &W, journey_id: &JourneyId, data: &DataBag) -> Result<Self, Error>;
}
