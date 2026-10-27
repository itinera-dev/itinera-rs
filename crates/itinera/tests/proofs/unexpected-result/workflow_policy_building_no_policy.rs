use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::policy::{OnWorkflowSuccess, Requested, WorkflowPolicyDescriptor, WorkflowSuccess};
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

struct Settle;

impl OnWorkflowSuccess<Orders> for Settle {
    fn on_workflow_success(
        &self,
        _got: Requested<'_, Orders, WorkflowSuccess>,
    ) -> Result<(), Error> {
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settle = WorkflowPolicyDescriptor::new("settle", || None::<Settle>).on_workflow_success();
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()));
    let orders = WorkflowDescriptor::builder("orders")
        .step(charge)
        .policy(settle)
        .build()?;
    LocalExecutor::new().run(orders.instance(Orders).create()?)?;
    Ok(())
}
