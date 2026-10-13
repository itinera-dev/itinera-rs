//! Fixtures the tests of several modules share: a workflow, a log of what happened on the test's
//! thread, reporters that log the keys of the global metadata they receive, and the workflow's
//! descriptor's builder.

use std::cell::RefCell;

use crate::error::Error;
use crate::event::Event;
use crate::report::Reporter;
use crate::value::AnyValue;
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

pub(super) fn note(reporter: &str, key: &str) {
    log(format!("{reporter} got {key}"));
}

pub(super) fn log(entry: String) {
    LOG.with_borrow_mut(|log| log.push(entry));
}

/// A reporter the workflow lists.
#[derive(Default)]
pub(super) struct Audit;

impl Reporter for Audit {
    fn set_global_metadata(&mut self, key: &str, _value: &AnyValue) -> Result<(), Error> {
        note("audit", key);
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
    fn set_global_metadata(&mut self, key: &str, _value: &AnyValue) -> Result<(), Error> {
        note(self.name, key);
        Ok(())
    }

    fn report(&mut self, _event: &Event) -> Result<(), Error> {
        Ok(())
    }
}

pub(super) fn orders() -> WorkflowBuilder<Orders> {
    WorkflowDescriptor::builder("orders")
}
