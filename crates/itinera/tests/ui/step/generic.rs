use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::Contributor;
use itinera::step::{DescribedStep, Outcome, StepReporter, step, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

struct Charge<T> {
    amount: T,
}

#[step]
impl<T: itinera::value::Value> Charge<T> {
    fn new(#[input] amount: T) -> Self {
        Self { amount }
    }

    fn run(
        self,
        contributor: &mut Contributor,
        mut reporter: StepReporter,
    ) -> Result<Outcome, Error> {
        reporter.info("charging")?;
        contributor.contribute("total", self.amount);
        Self::succeed()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders")
        .step(Charge::<i64>::descriptor(step_name!("charge")))
        .build()?;
    let instance = orders.instance(Orders).data("amount", 40_i64).create()?;
    LocalExecutor::new().run(instance)?;
    Ok(())
}
