use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::JourneyStatus;
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

#[derive(Step)]
struct Pack {}

#[step]
impl Pack {
    fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

#[derive(Step)]
struct Dispatch {}

#[step]
impl Dispatch {
    async fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

#[workflow(name = "delivery")]
#[with_step(Pack, name = "pack")]
#[with_step(Dispatch, name = "dispatch")]
struct Delivery;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Delivery::builder().create()?;
    let result = LocalExecutor::new().run(instance)?;
    assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    Ok(())
}
