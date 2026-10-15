use itinera::executor::LocalExecutor;
use itinera::policy::{StepPolicy, step_policy};
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::{Provides, WorkflowDescriptor};

trait Desk {
    fn desk(&self) -> &'static str;
}

struct Orders;

impl Desk for Orders {
    fn desk(&self) -> &'static str {
        "orders desk"
    }
}

impl Provides<dyn Desk> for Orders {
    fn role(&self) -> &(dyn Desk + 'static) {
        self
    }
}

#[derive(Default)]
struct Audit;

#[step_policy]
impl Audit {
    fn checked(&self) -> bool {
        true
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
        .policy(Audit::descriptor());
    let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
    LocalExecutor::new().run(orders.instance(Orders).create()?)?;
    Ok(())
}
