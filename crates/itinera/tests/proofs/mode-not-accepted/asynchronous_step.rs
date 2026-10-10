use itinera::executor::LocalExecutor;
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::async_builder("orders")
        .step(StepDescriptor::new_async(step_name!("charge"), async || {
            Ok(Outcome::success())
        }))
        .build()?;
    let instance = orders.instance(Orders).create()?;
    LocalExecutor::new().run(instance)?;
    Ok(())
}
