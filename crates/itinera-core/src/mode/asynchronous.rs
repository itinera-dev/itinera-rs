use super::{Mode, sealed};
use crate::report::BoxedReporter;

/// The mode of a workflow with at least one asynchronous part, which only the asynchronous
/// executor accepts.
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
/// let orders: WorkflowDescriptor<Orders, Asynchronous> = WorkflowDescriptor::builder("orders")
///     .async_reporter::<Forward>()
///     .build();
/// # drop(orders);
/// ```
#[derive(Clone, Copy, Debug)]
pub enum Asynchronous {}

impl Mode for Asynchronous {
    type Reporter = BoxedReporter;
}

impl sealed::Sealed for Asynchronous {}
