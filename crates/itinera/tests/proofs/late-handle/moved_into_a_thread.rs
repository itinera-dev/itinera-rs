use std::thread;

use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::step::{Outcome, Resolved, Step, StepFactory, StepNeeds, StepReporter};

struct Charge<'a> {
    contributor: Contributor<'a>,
    reporter: StepReporter<'a>,
}

impl Step for Charge<'_> {
    fn run(self) -> Result<Outcome, Error> {
        let Self {
            mut contributor,
            mut reporter,
        } = self;
        thread::spawn(move || {
            contributor.contribute("stale", 1_i64);
            reporter.info("stale")
        });
        Ok(Outcome::success())
    }
}

struct ChargeFactory;

impl StepFactory for ChargeFactory {
    type Step<'a> = Charge<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().contributor().reporter()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charge<'a>, Error> {
        Ok(Charge {
            contributor: got.contributor()?,
            reporter: got.reporter()?,
        })
    }
}

fn main() {}
