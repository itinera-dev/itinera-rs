use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::policy::{OnSuccess, Provides, StepPolicy, step_policy};
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

trait Checks {
    fn checked(&self, receipt: Option<String>) -> Result<Option<OnSuccess>, Error>;
}

#[step_policy]
impl Checks for Audit {
    fn checked(&self, receipt: Option<String>) -> Result<Option<OnSuccess>, Error> {
        match receipt {
            Some(_) => Self::carry_on(),
            None => Self::finish_workflow(),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
        .policy(Audit::descriptor());
    let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
    LocalExecutor::new().run(orders.instance(Orders).create()?)?;
    Ok(())
}
