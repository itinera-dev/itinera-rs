use itinera::executor::LocalExecutor;
use itinera::step::{StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(()));
    let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
    LocalExecutor::new().run(orders.instance(Orders).create()?)?;
    Ok(())
}
