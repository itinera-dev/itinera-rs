//! The workflows the tests of scripted runs script: an orders workflow whose charge step is
//! retried or rescued, followed by a ship step.

use super::{AttemptScripting, Named, Opened, Scripted, StepScripting};
use crate::instance::{Instance, WorkflowInstance};
use crate::mode::Synchronous;
use crate::step::{Outcome, RescueOutcome, StepDescriptor, StepName};
use crate::workflow::WorkflowBuilder;
use crate::workflow::fixtures::{Orders, orders};

pub(super) type Orderly = Scripted<Instance<Orders>>;

pub(super) type OrderlyStep = StepScripting<Instance<Orders>, Synchronous>;

/// The script of a step of the orders workflow, just named.
pub(super) type Charge = AttemptScripting<Instance<Orders>, Synchronous, Named>;

/// An attempt of a step of the orders workflow, chosen and not yet described.
pub(super) type Charging = AttemptScripting<Instance<Orders>, Synchronous, Opened>;

fn step(name: &'static str) -> StepDescriptor<Orders> {
    StepDescriptor::new(StepName::new(name), || Ok(Outcome::success()))
}

/// The orders workflow, whose charge step has this retry budget, then ships.
pub(super) fn retried(budget: u16) -> Orderly {
    scripted(
        orders()
            .step(step("charge").retry_budget(budget))
            .step(step("ship")),
    )
}

/// The orders workflow, whose charge step has this retry budget and does not retry an abnormal
/// termination, then ships.
pub(super) fn not_retried_abnormally(budget: u16) -> Orderly {
    let charge = step("charge")
        .retry_budget(budget)
        .no_retry_on_abnormal_termination();
    scripted(orders().step(charge).step(step("ship")))
}

/// The orders workflow, whose charge step has a rescue step that succeeds and does not retry an
/// abnormal termination, then ships.
pub(super) fn rescued() -> Orderly {
    let charge = step("charge")
        .no_retry_on_abnormal_termination()
        .rescue(|| Ok(RescueOutcome::success()));
    scripted(orders().step(charge).step(step("ship")))
}

fn scripted(workflow: WorkflowBuilder<Orders>) -> Orderly {
    workflow
        .build()
        .unwrap()
        .instance(Orders)
        .create()
        .unwrap()
        .scripted()
}
