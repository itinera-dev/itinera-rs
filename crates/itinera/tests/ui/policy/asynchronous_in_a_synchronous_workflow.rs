use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::policy::{AsyncHookReporter, OnSuccess, StepPolicy, step_policy};
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

#[derive(Default)]
struct Audit;

#[step_policy]
impl Audit {
    #[on_step_success]
    async fn checked(
        &self,
        mut reporter: AsyncHookReporter<'_>,
    ) -> Result<OnSuccess, Error> {
        reporter.info("checked").await?;
        Self::keep_default()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
        .policy(Audit::descriptor());
    let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
    LocalExecutor::new().run(orders.instance(Orders).create()?)?;
    Ok(())
}
