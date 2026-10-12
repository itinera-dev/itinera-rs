use itinera::error::Error;
use itinera::step::{Outcome, Resolved, Step, StepFactory, StepNeeds, StepReporter};

struct Lookup<'a> {
    reporter: StepReporter<'a>,
}

impl Step for Lookup<'_> {
    fn run(mut self) -> Result<Outcome, Error> {
        self.reporter.set_global_metadata("callback", 1_i64)?;
        Ok(Outcome::success())
    }
}

struct LookupFactory;

impl StepFactory for LookupFactory {
    type Step<'a> = Lookup<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().reporter()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Lookup<'a>, Error> {
        Ok(Lookup {
            reporter: got.reporter()?,
        })
    }
}

fn main() {}
