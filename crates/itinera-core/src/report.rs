//! Reporters, which receive the events and the global metadata of a journey, and dispatchers,
//! which deliver them.

use crate::error::Error;
use crate::event::Event;
use crate::mode::Mode;
use crate::value::AnyValue;

#[cfg(feature = "async")]
mod asynchronous;
mod debug_reporter;
mod default_dispatcher;
mod dispatcher;
#[cfg(test)]
pub(crate) mod fixtures;
mod global_metadata;
#[cfg(any(test, feature = "testing"))]
mod recording_reporter;
pub(crate) mod sealed;

#[cfg(feature = "async")]
pub use asynchronous::{AsyncDispatcher, AsyncDispatcherFactory, AsyncReporter, BoxedReporter};
pub use debug_reporter::DebugReporter;
pub use default_dispatcher::{DefaultDispatcher, DefaultDispatcherFactory};
pub use dispatcher::{Dispatcher, DispatcherFactory};
pub use global_metadata::GlobalMetadata;
#[cfg(any(test, feature = "testing"))]
pub use recording_reporter::RecordingReporter;

/// Receives the events of a journey, and decides on its own what to do with each: filter it,
/// map it, forward it or ignore it.
///
/// Each journey gets its own reporters: those its workflow lists, made with `Default` for every
/// instance, then those the application makes for that one instance and adds as it creates it.
/// Besides events, each receives the journey's global metadata with
/// [`set_global_metadata`](Reporter::set_global_metadata), the journey ID first, before the
/// journey starts.
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
/// use itinera::error::Result;
/// use itinera::event::Event;
/// use itinera::report::Reporter;
///
/// #[derive(Default)]
/// struct Kinds {
///     kinds: Vec<&'static str>,
/// }
///
/// impl Reporter for Kinds {
///     fn report(&mut self, event: &Event) -> Result<()> {
///         self.kinds.push(event.kind());
///         Ok(())
///     }
/// }
///
/// let reporter: Box<dyn Reporter> = Box::new(Kinds::default());
/// ```
pub trait Reporter: Send + 'static {
    /// Receives one entry of the journey's global metadata: first the journey ID, under
    /// [`GlobalMetadata::JOURNEY_ID`], and the instance's initial global metadata, before any
    /// event, then each entry a step, hook or service provider sets, as soon as it is set. What
    /// the reporter does with it is its own decision; one that keeps the entries, to use with
    /// the events that follow, can keep them in a [`GlobalMetadata`]. It does nothing unless the
    /// reporter says otherwise.
    ///
    /// # Errors
    ///
    /// An error on an entry set before the journey starts refuses the journey, with
    /// [`Refusal::GlobalMetadata`](crate::executor::Refusal::GlobalMetadata). An error on an
    /// entry set while it runs aborts it with `reporter failed`, as on an event.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::event::Event;
    /// use itinera::report::{GlobalMetadata, Reporter};
    /// use itinera::value::AnyValue;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// #[derive(Default)]
    /// struct Audit {
    ///     metadata: GlobalMetadata,
    /// }
    ///
    /// impl Reporter for Audit {
    ///     fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<()> {
    ///         self.metadata.insert(key, value.clone());
    ///         Ok(())
    ///     }
    ///
    ///     fn report(&mut self, event: &Event) -> Result<()> {
    ///         let journey = self.metadata.get(GlobalMetadata::JOURNEY_ID);
    ///         eprintln!("{journey:?} {}", event.kind());
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders").reporter::<Audit>();
    /// ```
    fn set_global_metadata(&mut self, _key: &str, _value: &AnyValue) -> Result<(), Error> {
        Ok(())
    }

    /// Receives one event.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::event::Event;
    /// use itinera::report::Reporter;
    ///
    /// struct Silent;
    ///
    /// impl Reporter for Silent {
    ///     fn report(&mut self, _event: &Event) -> Result<()> {
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
