use itinera::executor::LocalExecutor;
use itinera::workflow::WorkflowDescriptor;

struct Orders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders").build()?;
    let instance = orders.instance(Orders).create()?;
    let mut executor = LocalExecutor::new();
    executor.run(instance)?;
    executor.run(orders.instance(Orders).create()?)?;
    Ok(())
}
