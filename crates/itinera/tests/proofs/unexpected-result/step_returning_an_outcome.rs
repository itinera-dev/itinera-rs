use itinera::executor::LocalExecutor;
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()));
    let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
    LocalExecutor::new().run(orders.instance(Orders).create()?)?;
    Ok(())
}
