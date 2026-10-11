//! Dispatchers, which deliver the events of one journey to its reporters, and their factories.

use super::Reporter;
use crate::error::Error;
use crate::event::Event;

/// Delivers the events of one journey to the reporters added to it.
///
/// An executor gets a new dispatcher for every journey from its [`DispatcherFactory`], adds the
/// journey's reporters to it, dispatches every event of the journey through it, and drops it
/// when the journey ends. What adding a reporter means, and how events are delivered, is the
/// dispatcher's own decision; [`DefaultDispatcher`] is the usual one.
///
/// Failing in [`add`](Dispatcher::add) refuses the journey before it starts; failing in
/// [`dispatch`](Dispatcher::dispatch) aborts it with `reporter failed`, except while
/// `journey_aborted` is dispatched, when the error is ignored.
///
/// [`DefaultDispatcher`]: super::DefaultDispatcher
///
/// # Examples
///
/// A dispatcher that ignores the workflow's reporters and delivers only to its own:
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::report::{Dispatcher, Reporter};
///
/// struct OnlyMine {
///     reporter: Box<dyn Reporter>,
/// }
///
/// impl Dispatcher for OnlyMine {
///     fn add(&mut self, _reporter: Box<dyn Reporter>) -> Result<(), Error> {
///         Ok(())
///     }
///
///     fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
///         self.reporter.report(event)
///     }
/// }
/// ```
pub trait Dispatcher: Send + 'static {
    /// Adds a reporter for the journey.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::{DefaultDispatcher, Dispatcher, Reporter};
    ///
    /// struct Silent;
    ///
    /// impl Reporter for Silent {
    ///     fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut dispatcher = DefaultDispatcher::new();
    /// assert!(dispatcher.add(Box::new(Silent)).is_ok());
    /// ```
    fn add(&mut self, reporter: Box<dyn Reporter>) -> Result<(), Error>;

    /// Delivers one event.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::Dispatcher;
    ///
    /// fn deliver(dispatcher: &mut impl Dispatcher, event: &Event) -> Result<(), Error> {
    ///     dispatcher.dispatch(event)
    /// }
    /// ```
    fn dispatch(&mut self, event: &Event) -> Result<(), Error>;
}

/// Creates a new [`Dispatcher`] for every journey.
///
/// Failing refuses the journey before it starts.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::report::{DefaultDispatcher, DispatcherFactory};
///
/// struct Counting {
///     created: u32,
/// }
///
/// impl DispatcherFactory for Counting {
///     type Dispatcher = DefaultDispatcher;
///
///     fn create(&mut self) -> Result<DefaultDispatcher, Error> {
///         self.created += 1;
///         Ok(DefaultDispatcher::new())
///     }
/// }
///
/// let mut factory = Counting { created: 0 };
/// assert!(factory.create().is_ok());
/// assert_eq!(factory.created, 1);
/// ```
pub trait DispatcherFactory: Send + 'static {
    /// The dispatchers it creates.
    type Dispatcher: Dispatcher;

    /// Creates the dispatcher for one journey.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::report::{DefaultDispatcherFactory, DispatcherFactory};
    ///
    /// assert!(DefaultDispatcherFactory.create().is_ok());
    /// ```
    fn create(&mut self) -> Result<Self::Dispatcher, Error>;
}
