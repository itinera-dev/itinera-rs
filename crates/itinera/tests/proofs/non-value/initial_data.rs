use itinera::workflow::WorkflowDescriptor;

struct Orders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders").build()?;
    let _instance = orders.instance(Orders).data("callback", || 1).create();
    Ok(())
}
