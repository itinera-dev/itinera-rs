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

#[derive(Default)]
struct Log;

impl itinera::report::AsyncReporter for Log {
    async fn report(&mut self, _event: &itinera::event::Event) -> Result<(), Error> {
        Ok(())
    }
}

#[workflow(name = "delivery", async_reporters = [Log])]
#[with_step(Dispatch, name = "dispatch")]
struct Delivery;

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Delivery::builder().async_reporter(Log).create()?;
    let result = futures::executor::block_on(AsyncLocalExecutor::new().run(instance))?;
    assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    Ok(())
}
