use itinera::error::Error;
use itinera::executor::local_executor_for;
use itinera::journey::StatusKind;
use itinera::report::{
    AsyncDispatcherFactory, BoxedReporter, DefaultDispatcher, DefaultDispatcherFactory,
};
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

struct Forwarding;

impl AsyncDispatcherFactory for Forwarding {
    type Dispatcher = DefaultDispatcher<BoxedReporter>;

    async fn create(&mut self) -> Result<Self::Dispatcher, Error> {
        AsyncDispatcherFactory::create(&mut DefaultDispatcherFactory).await
    }
}

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::async_builder("orders")
        .step(StepDescriptor::new_async(
            step_name!("charge"),
            async || Ok(Outcome::success()),
        ))
        .build()?;
    let instance = orders.instance(Orders).create()?;
    let mut executor = local_executor_for!(instance, dispatcher_factory = Forwarding);
    let result = futures::executor::block_on(executor.run(instance))?;
    assert_eq!(result.status.kind(), StatusKind::Succeeded);
    Ok(())
}
