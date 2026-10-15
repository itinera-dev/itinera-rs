use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::step::{DescribedStep, Outcome, Step, StepReporter, step, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    fn run(
        self,
        receipts: &mut Vec<String>,
        mut reporter: StepReporter,
    ) -> Result<Outcome, Error> {
        reporter.info("charging")?;
        receipts.push(self.amount.to_string());
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
