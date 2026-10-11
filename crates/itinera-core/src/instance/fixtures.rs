//! Fixtures the tests of several modules share: a workflow, a reporter it makes for each
//! journey, and its descriptor's builder.

use std::sync::{Arc, Mutex};

use crate::error::Error;
use crate::event::Event;
use crate::journey::{DataBag, JourneyId};
use crate::report::{Reporter, WorkflowReporter};
use crate::workflow::{WorkflowBuilder, WorkflowDescriptor};

pub(super) struct Orders {
    pub(super) made: Arc<Mutex<Vec<String>>>,
}

impl Orders {
    pub(super) fn new() -> Self {
        Self {
            made: Arc::default(),
        }
    }

    pub(super) fn made(&self) -> Vec<String> {
        self.made.lock().unwrap().clone()
    }
}

pub(super) struct Audit;

impl Reporter for Audit {
    fn report(&mut self, _event: &Event) -> Result<(), Error> {
        Ok(())
    }
}

impl WorkflowReporter<Orders> for Audit {
    fn init(orders: &Orders, journey_id: &JourneyId, data: &DataBag) -> Result<Self, Error> {
        let keys: Vec<&str> = data.keys().collect();
        orders
            .made
            .lock()
            .unwrap()
            .push(format!("audit for {journey_id} with {keys:?}"));
        Ok(Audit)
    }
}

pub(super) fn orders() -> WorkflowBuilder<Orders> {
    WorkflowDescriptor::builder("orders")
}
