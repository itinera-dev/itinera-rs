use itinera::executor::LocalExecutor;
use itinera::step::{DescribedStep, Step, step, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    fn total(&self) -> i64 {
        self.amount
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders")
        .step(Charge::descriptor(step_name!("charge")))
        .build()?;
    let instance = orders.instance(Orders).data("amount", 40_i64).create()?;
    LocalExecutor::new().run(instance)?;
    Ok(())
}
