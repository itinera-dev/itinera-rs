use std::future::Future;

use crate::error::Error;
use crate::event::Event;
use crate::journey::JourneyId;

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
    /// Receives the ID of the journey the reporter is for, once, as the workflow instance is
    /// created and before the journey starts. It does nothing unless the reporter says otherwise.
    /// It is synchronous, since creating an instance is.
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
    /// use itinera::report::AsyncReporter;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// #[derive(Default)]
    /// struct Forward {
    ///     journey: Option<JourneyId>,
    /// }
    ///
    /// impl AsyncReporter for Forward {
    ///     fn set_journey_id(&mut self, journey_id: &JourneyId) -> Result<(), Error> {
    ///         self.journey = Some(journey_id.clone());
    ///         Ok(())
    ///     }
    ///
    ///     async fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders, _>::async_builder("orders")
    ///     .async_reporter::<Forward>();
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
