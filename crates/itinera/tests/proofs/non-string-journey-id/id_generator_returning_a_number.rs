use itinera::workflow::WorkflowDescriptor;

struct Orders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders")
        .id_generator(|_: &Orders, _| Ok(7_i64))
        .build()?;
    orders.instance(Orders).create()?;
    Ok(())
}
