use itinera::error::Error;
use itinera::executor::AsyncLocalExecutor;
use itinera::journey::JourneyStatus;
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

#[derive(Step)]
struct Dispatch {}

#[step]
impl Dispatch {
    async fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

#[workflow(name = "delivery")]
#[with_step(Dispatch, name = "dispatch")]
struct Delivery;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Delivery::builder().create()?;
    let result = futures::executor::block_on(AsyncLocalExecutor::new().run(instance))?;
    assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    Ok(())
}
