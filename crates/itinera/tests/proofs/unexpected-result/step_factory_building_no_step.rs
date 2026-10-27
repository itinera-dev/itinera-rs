use itinera::error::Result;
use itinera::executor::LocalExecutor;
use itinera::step::{Outcome, Resolved, Step, StepDescriptor, StepFactory, StepNeeds, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

struct Charge;

impl Step for Charge {
    fn run(self) -> Result<Outcome> {
        Ok(Outcome::success())
    }
}

struct ChargeFactory;

impl StepFactory for ChargeFactory {
    type Step<'a> = Option<Charge>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
    }

    fn build<'a>(&'a self, _got: &mut Resolved<'a>) -> Result<Option<Charge>> {
        Ok(None)
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let charge = StepDescriptor::new(step_name!("charge"), ChargeFactory);
    let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
    LocalExecutor::new().run(orders.instance(Orders).create()?)?;
    Ok(())
}
