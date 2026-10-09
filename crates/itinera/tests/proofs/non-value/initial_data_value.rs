use itinera::workflow::WorkflowDescriptor;

struct Orders;

fn main() {
    let orders = WorkflowDescriptor::builder("orders").build().unwrap();
    let _instance = orders.instance(Orders).data("callback", 1_i64).create();
}
