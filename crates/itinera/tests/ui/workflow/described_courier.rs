use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

#[derive(Step)]
struct Ship {}

#[step]
impl Ship {
    fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

#[workflow(name = "courier")]
#[with_step(Ship, name = "ship")]
struct Courier {
    #[default]
    #[expect(dead_code, reason = "this field only has to have a name")]
    van: &'static str,
}

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Courier::builder().create()?;
    let result = LocalExecutor::new().run(instance)?;
    assert!(result.status.is_succeeded());
    Ok(())
}
