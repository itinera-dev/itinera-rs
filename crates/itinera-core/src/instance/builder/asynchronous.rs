//! Adding an asynchronous reporter to an instance of an asynchronous workflow.

use super::InstanceBuilder;
use crate::mode::Asynchronous;
use crate::report::{AsyncReporter, BoxedReporter};

impl<W: Send + Sync + 'static> InstanceBuilder<W, Asynchronous> {
    /// Adds an asynchronous reporter made for this journey alone. It receives events after the
    /// reporters the workflow lists, and after those added before it.
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
    /// struct Forward {
    ///     request_id: String,
    /// }
    ///
    /// impl AsyncReporter for Forward {
    ///     async fn report(&mut self, event: &Event) -> Result<(), Error> {
    ///         println!("[{}] {}", self.request_id, event.kind());
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let orders = WorkflowDescriptor::async_builder("orders").build()?;
    /// let forward = Forward {
    ///     request_id: "req-7".to_string(),
    /// };
    /// let instance = orders.instance(Orders).async_reporter(forward).create()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn async_reporter(mut self, reporter: impl AsyncReporter) -> Self {
        self.reporters
            .push(BoxedReporter::from_async_reporter(reporter));
        self
    }
}
