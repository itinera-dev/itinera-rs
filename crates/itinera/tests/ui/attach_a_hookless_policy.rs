use itinera::error::Error;
use itinera::policy::{OnStepSuccess, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess};
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

struct Audit;

impl OnStepSuccess<Orders> for Audit {
    fn on_step_success(
        &self,
        _got: Requested<'_, Orders, StepSuccess>,
    ) -> Result<Option<OnSuccess>, Error> {
        Ok(None)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let audit = StepPolicyDescriptor::new("audit", || Audit);
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())).policy(audit);
    WorkflowDescriptor::<Orders>::builder("orders")
        .step(charge)
        .build()?;
    Ok(())
}
