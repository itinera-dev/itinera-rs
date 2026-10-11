use itinera::executor::LocalExecutor;
use itinera::journey::{InitialData, Output};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

#[derive(InitialData, Output)]
struct Order {
    price: i64,
    #[key("discount_code")]
    code: Option<String>,
    #[required]
    gift_note: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders").build()?;
    let order = Order {
        price: 42,
        code: None,
        gift_note: None,
    };
    let instance = orders.instance(Orders).initial_data(order).create()?;
    let order = LocalExecutor::new().run(instance)?.output::<Order>()?;
    assert_eq!(order.price, 42);
    assert_eq!(order.code, None);
    assert_eq!(order.gift_note, None);
    Ok(())
}
