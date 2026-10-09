use std::sync::Mutex;

use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::step::{Outcome, Resolved, Step, StepFactory, StepNeeds, StepReporter};

struct Charge {
    kept: Mutex<Option<(Contributor<'static>, StepReporter<'static>)>>,
}

impl StepFactory for Charge {
    type Step<'a> = Charged;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().contributor().reporter()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charged, Error> {
        let handles = (got.contributor()?, got.reporter()?);
        *self.kept.lock().unwrap() = Some(handles);
        Ok(Charged)
    }
}

struct Charged;

impl Step for Charged {
    fn run(self) -> Result<Outcome, Error> {
        Ok(Outcome::success())
    }
}

fn main() {}
