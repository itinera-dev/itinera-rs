//! Dispatchers, which deliver the events and the global metadata of one journey to its
//! reporters, and their factories.

use super::Reporter;
use crate::error::Error;
use crate::event::Event;
use crate::value::AnyValue;

/// Delivers the events and the global metadata of one journey to the reporters added to it.
///
/// An executor gets a new dispatcher for every journey from its [`DispatcherFactory`], adds the
/// journey's reporters to it, sets every entry of the journey's global metadata and dispatches
/// every event of the journey through it, and drops it when the journey ends. What adding a
/// reporter means, and how events and global metadata are delivered, is the dispatcher's own
/// decision; [`DefaultDispatcher`] is the usual one.
///
/// Failing in [`add`](Dispatcher::add), or in
/// [`set_global_metadata`](Dispatcher::set_global_metadata) before the journey starts, refuses
/// the journey; failing in [`dispatch`](Dispatcher::dispatch), or in `set_global_metadata` while
/// the journey runs, aborts it with `reporter failed`, except while `journey_aborted` is
/// dispatched, when the error is ignored.
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
/// use itinera::value::AnyValue;
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
///
///     fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
///         self.reporter.set_global_metadata(key, value)
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

    /// Passes one entry of the journey's global metadata on to the reporters it holds.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::report::Dispatcher;
    /// use itinera::value::AnyValue;
    ///
    /// fn tell(dispatcher: &mut impl Dispatcher, user: &str) -> Result<(), Error> {
    ///     dispatcher.set_global_metadata("user_id", &AnyValue::new(user.to_string()))
    /// }
    /// ```
    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error>;
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
