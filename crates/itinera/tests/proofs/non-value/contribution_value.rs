use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::step::{Outcome, Resolved, Step, StepFactory, StepNeeds};

struct Charge<'a> {
    contributor: Contributor<'a>,
}

impl Step for Charge<'_> {
    fn run(mut self) -> Result<Outcome, Error> {
        self.contributor.contribute("callback", 1_i64);
        Ok(Outcome::success())
    }
}

struct ChargeFactory;

impl StepFactory for ChargeFactory {
    type Step<'a> = Charge<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().contributor()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charge<'a>, Error> {
        Ok(Charge {
            contributor: got.contributor()?,
        })
    }
}

fn main() {}
