//! Fixtures the tests of several modules share: a workflow, its steps, its input adapters and
//! its descriptor's builder.

use super::{InputAdapterDescriptor, WorkflowBuilder, WorkflowDescriptor};
use crate::step::{Outcome, StepDescriptor, StepName};

pub(crate) struct Orders;

pub(super) fn step(name: &'static str) -> StepDescriptor<Orders> {
    StepDescriptor::new(StepName::new(name), || Ok(Outcome::success()))
}

pub(super) fn adapter(name: &'static str, step: &'static str) -> InputAdapterDescriptor<Orders> {
    InputAdapterDescriptor::new(name, StepName::new(step), |_, _| Ok(None))
}

pub(crate) fn orders() -> WorkflowBuilder<Orders> {
    WorkflowDescriptor::builder("orders")
}
