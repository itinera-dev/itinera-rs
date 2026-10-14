use itinera::executor::local_executor_for;
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders")
        .step(StepDescriptor::new(step_name!("charge"), || {
            Ok(Outcome::success())
        }))
        .build()?;
    let mut executor = local_executor_for!(Orders);
    executor.run(orders.instance(Orders).create()?)?;
    Ok(())
}
