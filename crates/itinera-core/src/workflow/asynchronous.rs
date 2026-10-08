use super::{Declaration, MakeReporter, WorkflowBuilder};
use crate::error::Error;
use crate::journey::{DataBag, JourneyId};
use crate::mode::{Asynchronous, Synchronous};
use crate::report::{AsyncWorkflowReporter, BoxedReporter};

impl<W: Send + Sync + 'static> WorkflowBuilder<W, Synchronous> {
    /// Lists an asynchronous reporter, which each instance makes for its journey, and makes the
    /// workflow asynchronous. Reporters receive events in the order they are listed.
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
    pub fn async_reporter<R: AsyncWorkflowReporter<W>>(self) -> WorkflowBuilder<W, Asynchronous> {
        self.asynchronous().async_reporter::<R>()
    }

    fn asynchronous(self) -> WorkflowBuilder<W, Asynchronous> {
        let Declaration {
            name,
            reporters,
            id_generator,
        } = self.declaration;
        WorkflowBuilder {
            declaration: Declaration {
                name,
                reporters: reporters.into_iter().map(asynchronous).collect(),
                id_generator,
            },
        }
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
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .async_reporter::<Forward>()
    ///     .async_reporter::<Forward>()
    ///     .build();
    /// # drop(orders);
    /// ```
    pub fn async_reporter<R: AsyncWorkflowReporter<W>>(mut self) -> Self {
        self.declaration
            .reporters
            .push(Box::new(make_async_reporter::<W, R>));
        self
    }
}

/// A synchronous workflow's way of making a reporter, as an asynchronous workflow holds it.
fn asynchronous<W: 'static>(make: MakeReporter<W, Synchronous>) -> MakeReporter<W, Asynchronous> {
    Box::new(move |workflow, journey_id, data| {
        make(workflow, journey_id, data).map(BoxedReporter::from)
    })
}

fn make_async_reporter<W, R: AsyncWorkflowReporter<W>>(
    workflow: &W,
    journey_id: &JourneyId,
    data: &DataBag,
) -> Result<BoxedReporter, Error> {
    R::init(workflow, journey_id, data).map(BoxedReporter::from_async_reporter)
}
