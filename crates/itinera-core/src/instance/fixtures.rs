//! Fixtures the tests of several modules share: a workflow, a log of what happened on the test's
//! thread, reporters that log the journey ID they are given, and the workflow's descriptor's
//! builder.

use std::cell::RefCell;

use crate::error::Error;
use crate::event::Event;
use crate::journey::JourneyId;
use crate::report::Reporter;
use crate::workflow::{WorkflowBuilder, WorkflowDescriptor};

pub(super) struct Orders {
    /// Which shop the orders are for, so that a test can tell the workflow's value apart.
    pub(super) shop: &'static str,
}

impl Orders {
    pub(super) fn new() -> Self {
        Self { shop: "north" }
    }
}

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// What was logged on this thread, in order.
pub(super) fn entries() -> Vec<String> {
    LOG.with_borrow(Clone::clone)
}

pub(super) fn note(reporter: &str, journey_id: &JourneyId) {
    log(format!("{reporter} got {journey_id}"));
}

pub(super) fn log(entry: String) {
    LOG.with_borrow_mut(|log| log.push(entry));
}

/// A reporter the workflow lists.
#[derive(Default)]
pub(super) struct Audit;

impl Reporter for Audit {
    fn set_journey_id(&mut self, journey_id: &JourneyId) -> Result<(), Error> {
        note("audit", journey_id);
        Ok(())
    }

    fn report(&mut self, _event: &Event) -> Result<(), Error> {
        Ok(())
    }
}

/// A reporter made for one instance.
pub(super) struct Made {
    pub(super) name: &'static str,
}

impl Reporter for Made {
    fn set_journey_id(&mut self, journey_id: &JourneyId) -> Result<(), Error> {
        note(self.name, journey_id);
        Ok(())
    }

    fn report(&mut self, _event: &Event) -> Result<(), Error> {
        Ok(())
    }
}

pub(super) fn orders() -> WorkflowBuilder<Orders> {
    WorkflowDescriptor::builder("orders")
}
