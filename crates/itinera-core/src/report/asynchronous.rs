use std::future::Future;

use crate::error::Error;
use crate::event::Event;
use crate::value::AnyValue;

mod boxed_reporter;
mod dispatcher;

pub use boxed_reporter::BoxedReporter;
pub use dispatcher::{AsyncDispatcher, AsyncDispatcherFactory};

/// A [`Reporter`] whose `report` is asynchronous. A workflow with one is asynchronous.
///
/// What [`Reporter`] says about the reporters of a journey and about failing applies to it too.
///
/// [`Reporter`]: super::Reporter
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::event::Event;
/// use itinera::report::AsyncReporter;
///
/// struct Forward;
///
/// impl AsyncReporter for Forward {
///     async fn report(&mut self, _event: &Event) -> Result<()> {
///         Ok(())
///     }
/// }
/// ```
pub trait AsyncReporter: Send + 'static {
    /// Receives one entry of the journey's global metadata, as
    /// [`Reporter::set_global_metadata`](super::Reporter::set_global_metadata) does. It does
    /// nothing unless the reporter says otherwise. It is synchronous: the entry is for the events
    /// that follow, which are asynchronous.
    ///
    /// # Errors
    ///
    /// As for [`Reporter::set_global_metadata`](super::Reporter::set_global_metadata).
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::event::Event;
    /// use itinera::report::{AsyncReporter, GlobalMetadata};
    /// use itinera::value::AnyValue;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// #[derive(Default)]
    /// struct Forward {
    ///     metadata: GlobalMetadata,
    /// }
    ///
    /// impl AsyncReporter for Forward {
    ///     fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<()> {
    ///         self.metadata.insert(key, value.clone());
    ///         Ok(())
    ///     }
    ///
    ///     async fn report(&mut self, _event: &Event) -> Result<()> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders, _>::async_builder("orders")
    ///     .async_reporter::<Forward>();
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
    /// use itinera::report::AsyncReporter;
    ///
    /// struct Silent;
    ///
    /// impl AsyncReporter for Silent {
    ///     async fn report(&mut self, _event: &Event) -> Result<()> {
    ///         Ok(())
    ///     }
    /// }
    /// ```
    fn report(&mut self, event: &Event) -> impl Future<Output = Result<(), Error>> + Send;
}
