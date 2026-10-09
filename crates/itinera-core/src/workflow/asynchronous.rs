use super::{WorkflowBuilder, WorkflowDescriptor, WorkflowName};
use crate::error::Error;
use crate::journey::{DataBag, JourneyId};
use crate::mode::Asynchronous;
use crate::report::{AsyncWorkflowReporter, BoxedReporter};

impl<W: Send + Sync + 'static> WorkflowDescriptor<W, Asynchronous> {
    /// Starts declaring an asynchronous workflow with this name, which only the asynchronous
    /// executor runs. Its steps are asynchronous, and its reporters may be of either kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::mode::Asynchronous;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders: WorkflowDescriptor<Orders, Asynchronous> =
    ///     WorkflowDescriptor::async_builder("orders").build()?;
    /// assert_eq!(orders.name().to_string(), "orders");
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn async_builder(name: impl Into<WorkflowName>) -> WorkflowBuilder<W, Asynchronous> {
        WorkflowBuilder::named(name.into())
    }
}

impl<W: Send + Sync + 'static> WorkflowBuilder<W, Asynchronous> {
    /// Lists an asynchronous reporter, which each instance makes for its journey. Reporters
    /// receive events in the order they are listed.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::journey::{DataBag, JourneyId};
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
    /// let orders = WorkflowDescriptor::async_builder("orders")
    ///     .async_reporter::<Forward>()
    ///     .build()?;
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn async_reporter<R: AsyncWorkflowReporter<W>>(mut self) -> Self {
        self.declaration
            .reporters
            .push(Box::new(make_async_reporter::<W, R>));
        self
    }
}

fn make_async_reporter<W, R: AsyncWorkflowReporter<W>>(
    workflow: &W,
    journey_id: &JourneyId,
    data: &DataBag,
) -> Result<BoxedReporter, Error> {
    R::init(workflow, journey_id, data).map(BoxedReporter::from_async_reporter)
}
