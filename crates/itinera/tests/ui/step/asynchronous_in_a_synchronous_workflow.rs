use itinera::error::Error;
use itinera::executor::LocalExecutor;
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
    async fn run(self, #[reporter] mut reporter: AsyncStepReporter<'_>) -> Result<Outcome, Error> {
        reporter.info_with("charging", self.amount).await?;
        Self::succeed()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders")
        .step(Charge::descriptor(step_name!("charge")))
        .build()?;
    let instance = orders.instance(Orders).data("amount", 40_i64).create()?;
    LocalExecutor::new().run(instance)?;
    Ok(())
}
