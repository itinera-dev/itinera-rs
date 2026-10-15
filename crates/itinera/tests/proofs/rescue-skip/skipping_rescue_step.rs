use itinera::executor::LocalExecutor;
use itinera::step::{Outcome, RescueOutcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let charge = StepDescriptor::new(step_name!("charge"), || {
        Ok(Outcome::failure("expired"))
    })
    .rescue(|| Ok(RescueOutcome::skipped()));
    let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
    LocalExecutor::new().run(orders.instance(Orders).create()?)?;
    Ok(())
}
