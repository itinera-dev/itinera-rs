use futures::executor::block_on;
use itinera::executor::AsyncLocalExecutor;
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
    block_on(AsyncLocalExecutor::new().run(instance))?;
    Ok(())
}
