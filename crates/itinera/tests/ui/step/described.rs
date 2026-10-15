use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::Contributor;
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
        #[contributor] contributor: &mut Contributor,
        #[reporter] mut reporter: StepReporter<'_>,
    ) -> Result<Outcome, Error> {
        reporter.info("charging")?;
        contributor.contribute("total", self.amount);
        Self::succeed()
    }
}

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders")
        .step(Charge::descriptor(step_name!("charge")))
        .build()?;
    let instance = orders.instance(Orders).data("amount", 40_i64).create()?;
    LocalExecutor::new().run(instance)?;
    Ok(())
}
