use itinera::error::Error;
use itinera::step::{Outcome, Resolved, Step, StepFactory, StepNeeds, StepReporter};

struct Charge<'a> {
    reporter: StepReporter<'a>,
}

impl Step for Charge<'_> {
    fn run(mut self) -> Result<Outcome, Error> {
        self.reporter.info_with("odd data", 1_i64)?;
        Ok(Outcome::success())
    }
}

struct ChargeFactory;

impl StepFactory for ChargeFactory {
    type Step<'a> = Charge<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().reporter()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charge<'a>, Error> {
        Ok(Charge {
            reporter: got.reporter()?,
        })
    }
}

fn main() {}
