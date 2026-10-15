use itinera::error::Error;
use itinera::policy::{HookNeeds, OnFailure, OnStepFailure, Requested, StepFailure};

struct Orders;

struct Audit;

impl OnStepFailure<Orders> for Audit {
    fn needs() -> HookNeeds<StepFailure> {
        HookNeeds::new().reason()
    }

    fn on_step_failure(
        &self,
        mut got: Requested<'_, Orders, StepFailure>,
    ) -> Result<OnFailure, Error> {
        got.reason()?;
        Ok(OnFailure::KeepDefault)
    }
}

fn main() {}
