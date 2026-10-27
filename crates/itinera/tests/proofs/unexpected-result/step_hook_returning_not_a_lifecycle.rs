use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::policy::{OnStepFailure, Requested, StepFailure, StepPolicyDescriptor};
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

struct Settle;

impl OnStepFailure<Orders> for Settle {
    fn on_step_failure(
        &self,
        _got: Requested<'_, Orders, StepFailure>,
    ) -> Result<&'static str, Error> {
        Ok("settled")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settle = StepPolicyDescriptor::new("settle", || Settle).on_step_failure();
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::failure("declined")))
        .policy(settle);
    let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
    LocalExecutor::new().run(orders.instance(Orders).create()?)?;
    Ok(())
}
