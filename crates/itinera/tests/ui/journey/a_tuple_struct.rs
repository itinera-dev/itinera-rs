use itinera::executor::LocalExecutor;
use itinera::journey::{InitialData, Output};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

#[derive(InitialData, Output)]
struct Order(i64);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders").build()?;
    let instance = orders.instance(Orders).initial_data(Order(42)).create()?;
    let order = LocalExecutor::new().run(instance)?.output::<Order>()?;
    assert_eq!(order.0, 42);
    Ok(())
}
