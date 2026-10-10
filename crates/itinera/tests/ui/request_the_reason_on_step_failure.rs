use itinera::error::Error;
use itinera::policy::{FailWorkflow, HookNeeds, OnStepFailure, Requested, StepFailure};

struct Orders;

struct Audit;

impl OnStepFailure<Orders> for Audit {
    fn needs() -> HookNeeds<StepFailure> {
        HookNeeds::new().reason()
    }

    fn on_step_failure(
        &self,
        mut got: Requested<'_, Orders, StepFailure>,
    ) -> Result<Option<FailWorkflow>, Error> {
        got.reason()?;
        Ok(None)
    }
}

fn main() {}
