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
/// use itinera::error::Result;
/// use itinera::event::Event;
/// use itinera::mode::Asynchronous;
/// use itinera::report::AsyncReporter;
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// #[derive(Default)]
/// struct Forward;
///
/// impl AsyncReporter for Forward {
///     async fn report(&mut self, _event: &Event) -> Result<()> {
///         Ok(())
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
