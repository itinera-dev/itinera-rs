use itinera::error::Error;
use itinera::policy::{
    OnStepSuccess, OnSuccess, Provides, Requested, StepPolicyDescriptor, StepSuccess,
};
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

trait Notifier {
    fn notify(&self);
}

struct Orders;

impl Notifier for Orders {
    fn notify(&self) {}
}

impl Provides<dyn Notifier> for Orders {
    fn role(&self) -> &(dyn Notifier + 'static) {
        self
    }
}

struct Tell;

impl<W: Provides<dyn Notifier> + Send + Sync + 'static> OnStepSuccess<W> for Tell {
    fn on_step_success(
        &self,
        got: Requested<'_, W, StepSuccess>,
    ) -> Result<Option<OnSuccess>, Error> {
        got.role::<dyn Notifier>().notify();
        Ok(None)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tell = StepPolicyDescriptor::new("tell", || Tell).on_step_success();
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())).policy(tell);
    WorkflowDescriptor::<Orders>::builder("orders")
        .step(charge)
        .build()?;
    Ok(())
}
