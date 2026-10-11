use super::{WorkflowBuilder, WorkflowDescriptor, WorkflowName};
use crate::mode::Asynchronous;
use crate::report::{AsyncReporter, BoxedReporter};

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
    /// Lists an asynchronous reporter, which each instance makes for its journey with
    /// `Default`. Reporters receive events in the order they are listed, before those added as an
    /// instance is created.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::AsyncReporter;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// #[derive(Default)]
    /// struct Forward;
    ///
    /// impl AsyncReporter for Forward {
    ///     async fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let orders = WorkflowDescriptor::<Orders, _>::async_builder("orders")
    ///     .async_reporter::<Forward>()
    ///     .build()?;
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn async_reporter<R: AsyncReporter + Default>(mut self) -> Self {
        self.declaration.reporters.push(made_async::<R>);
        self
    }
}

fn made_async<R: AsyncReporter + Default>() -> BoxedReporter {
    BoxedReporter::from_async_reporter(R::default())
}
