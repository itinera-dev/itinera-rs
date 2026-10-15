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
struct Courier(&'static str);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Courier::builder().create()?;
    let result = LocalExecutor::new().run(instance)?;
    assert!(result.status.is_succeeded());
    Ok(())
}
