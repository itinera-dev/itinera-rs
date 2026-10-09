use super::{Mode, sealed};
use crate::report::BoxedReporter;

/// The mode of an asynchronous workflow, which only the asynchronous executor accepts.
///
/// Its steps are asynchronous, and await the delivery of the events they emit. Its reporters may
/// be of either kind.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::event::Event;
/// use itinera::journey::{DataBag, JourneyId};
/// use itinera::mode::Asynchronous;
/// use itinera::report::{AsyncReporter, AsyncWorkflowReporter};
/// use itinera::workflow::WorkflowDescriptor;
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
///
/// let orders: WorkflowDescriptor<Orders, Asynchronous> = WorkflowDescriptor::async_builder("orders")
///     .async_reporter::<Forward>()
///     .build()?;
/// # Ok::<(), itinera::workflow::Violations>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub enum Asynchronous {}

impl Mode for Asynchronous {
    type Reporter = BoxedReporter;
}

impl sealed::Sealed for Asynchronous {}
