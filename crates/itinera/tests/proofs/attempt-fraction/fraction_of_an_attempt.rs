use itinera::instance::WorkflowInstance;
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()));
    let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
    orders
        .instance(Orders)
        .create()?
        .scripted()
        .step("charge")
        .on_attempt(1.5)
        .will_succeed()
        .run()?;
    Ok(())
}
