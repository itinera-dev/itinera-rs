use std::thread;

use itinera::executor::LocalExecutor;
use itinera::workflow::WorkflowDescriptor;

struct Orders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders").build()?;
    let first = orders.instance(Orders).create()?;
    let second = orders.instance(Orders).create()?;
    let mut executor = LocalExecutor::new();
    thread::scope(|scope| {
        scope.spawn(|| executor.run(first));
        scope.spawn(|| executor.run(second));
    });
    Ok(())
}
