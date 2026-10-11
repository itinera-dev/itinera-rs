use itinera::workflow::WorkflowDescriptor;

struct Orders;

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders").build()?;
    let _instance = orders.instance(Orders).data("callback", 1_i64).create();
    Ok(())
}
