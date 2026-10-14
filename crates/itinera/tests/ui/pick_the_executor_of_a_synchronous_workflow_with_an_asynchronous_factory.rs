use itinera::error::Error;
use itinera::executor::local_executor_for;
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders")
        .step(StepDescriptor::new(step_name!("charge"), || {
            Ok(Outcome::success())
        }))
        .build()?;
    let instance = orders.instance(Orders).create()?;
    let _executor = local_executor_for!(instance, dispatcher_factory = Forwarding);
    Ok(())
}
