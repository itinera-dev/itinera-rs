use itinera::error::Error;
use itinera::policy::{HookNeeds, OnStepSuccess, OnSuccess, Requested, StepSuccess};

struct Orders;

struct Audit;

impl OnStepSuccess<Orders> for Audit {
    fn needs() -> HookNeeds<StepSuccess> {
        HookNeeds::new().reason()
    }

    fn on_step_success(
        &self,
        _got: Requested<'_, Orders, StepSuccess>,
    ) -> Result<Option<OnSuccess>, Error> {
        Ok(None)
    }
}

fn main() {}
