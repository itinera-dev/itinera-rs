use itinera::error::Error;
use itinera::executor::AsyncLocalExecutor;
use itinera::journey::Contributor;
use itinera::step::{Outcome, RescueOutcome, RescueStep, Step, rescue_step, step};
use itinera::workflow::workflow;

#[derive(Step)]
struct Dispatch {
    #[input]
    label: Option<String>,
}

#[step]
impl Dispatch {
    async fn run(self) -> Result<Outcome, Error> {
        match self.label {
            Some(_) => Self::succeed(),
            None => Self::fail("unlabelled"),
        }
    }
}

#[derive(RescueStep)]
struct Label;

#[rescue_step]
impl Label {
    async fn run(self, contributor: &mut Contributor) -> Result<RescueOutcome, Error> {
        contributor.contribute("label", "L-1".to_owned());
        Self::succeed()
    }
}

#[workflow(name = "delivery")]
#[with_step(Dispatch, name = "dispatch", rescue = Label)]
struct Delivery;

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Delivery::builder().create()?;
    let result = futures::executor::block_on(AsyncLocalExecutor::new().run(instance))?;
    assert!(result.status.is_succeeded());
    Ok(())
}
