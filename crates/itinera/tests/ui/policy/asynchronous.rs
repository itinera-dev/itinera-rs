use itinera::error::Error;
use itinera::executor::AsyncLocalExecutor;
use itinera::policy::{AsyncHookReporter, OnSuccess, StepPolicy, step_policy};
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

#[derive(Default)]
struct Audit;

#[step_policy]
impl Audit {
    #[on_step_success]
    async fn checked(&self, mut reporter: AsyncHookReporter<'_>) -> Result<OnSuccess, Error> {
        reporter.info("checked").await?;
        Self::keep_default()
    }
}

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let charge = StepDescriptor::new_async(step_name!("charge"), async || Ok(Outcome::success()))
        .policy(Audit::descriptor());
    let orders = WorkflowDescriptor::async_builder("orders")
        .step(charge)
        .build()?;
    futures::executor::block_on(AsyncLocalExecutor::new().run(orders.instance(Orders).create()?))?;
    Ok(())
}
