use itinera::error::Error;
use itinera::executor::AsyncLocalExecutor;
use itinera::step::{AsyncStepReporter, DescribedStep, Outcome, Step, step, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    async fn run(self, mut reporter: AsyncStepReporter<'_>) -> Result<Outcome, Error> {
        reporter.info_with("charging", self.amount).await?;
        Self::succeed()
    }
}

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::async_builder("orders")
        .step(Charge::descriptor(step_name!("charge")))
        .build()?;
    let instance = orders.instance(Orders).data("amount", 40_i64).create()?;
    futures::executor::block_on(AsyncLocalExecutor::new().run(instance))?;
    Ok(())
}
