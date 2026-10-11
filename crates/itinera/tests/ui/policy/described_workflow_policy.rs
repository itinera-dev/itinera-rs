use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::policy::{Provides, WorkflowPolicy, workflow_policy};
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

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

#[workflow_policy]
impl Audit {
    #[on_workflow_failure]
    fn reported(&self, #[role] desk: &dyn Desk) -> Result<(), Error> {
        println!("told the {}", desk.desk());
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()));
    let orders = WorkflowDescriptor::builder("orders")
        .step(charge)
        .policy(Audit::descriptor())
        .build()?;
    LocalExecutor::new().run(orders.instance(Orders).create()?)?;
    Ok(())
}
