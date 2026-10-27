use itinera::workflow::WorkflowDescriptor;

struct Orders;

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders")
        .id_generator(|_: &Orders, _| Ok("order-7".to_owned()))
        .build()?;
    orders.instance(Orders).create()?;
    Ok(())
}
