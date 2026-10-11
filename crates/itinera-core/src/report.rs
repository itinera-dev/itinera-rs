//! Reporters, which receive the events of a journey, and dispatchers, which deliver them.

use crate::error::Error;
use crate::event::Event;
use crate::journey::JourneyId;
use crate::mode::Mode;

#[cfg(feature = "async")]
mod asynchronous;
mod default_dispatcher;
mod dispatcher;
#[cfg(test)]
pub(crate) mod fixtures;
pub(crate) mod sealed;

#[cfg(feature = "async")]
pub use asynchronous::{AsyncDispatcher, AsyncDispatcherFactory, AsyncReporter, BoxedReporter};
pub use default_dispatcher::{DefaultDispatcher, DefaultDispatcherFactory};
pub use dispatcher::{Dispatcher, DispatcherFactory};

/// Receives the events of a journey, and decides on its own what to do with each: filter it,
/// map it, forward it or ignore it.
///
/// Each journey gets its own reporters: those its workflow lists, made with `Default` for every
/// instance, then those the application makes for that one instance and adds as it creates it.
/// Once the journey ID exists, each is given it with [`set_journey_id`](Reporter::set_journey_id),
/// before the journey starts.
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
    /// Receives the ID of the journey the reporter is for, once, as the workflow instance is
    /// created and before the journey starts. It does nothing unless the reporter says otherwise.
    ///
    /// # Errors
    ///
    /// An error makes creating the instance fail, with
    /// [`InstanceError::Reporter`](crate::instance::InstanceError::Reporter).
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::journey::JourneyId;
    /// use itinera::report::Reporter;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// #[derive(Default)]
    /// struct Audit {
    ///     journey: Option<JourneyId>,
    /// }
    ///
    /// impl Reporter for Audit {
    ///     fn set_journey_id(&mut self, journey_id: &JourneyId) -> Result<(), Error> {
    ///         self.journey = Some(journey_id.clone());
    ///         Ok(())
    ///     }
    ///
    ///     fn report(&mut self, event: &Event) -> Result<(), Error> {
    ///         eprintln!("{:?} {}", self.journey, event.kind());
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders").reporter::<Audit>();
    /// ```
    fn set_journey_id(&mut self, _journey_id: &JourneyId) -> Result<(), Error> {
        Ok(())
    }

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

/// A reporter as the mode `M` holds it.
pub(crate) fn held<M: Mode>(reporter: impl Reporter) -> M::Reporter {
    let reporter: Box<dyn Reporter> = Box::new(reporter);
    M::Reporter::from(reporter)
}
