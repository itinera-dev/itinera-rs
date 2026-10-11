use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::step::{DescribedStep, Outcome, step, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Orders;

struct Ship {
    gift_note: Option<String>,
}

#[step]
impl Ship {
    fn new(#[input] gift_note: Option<String>) -> Result<Self, Error> {
        Ok(Self { gift_note })
    }

    fn run(self) -> Result<Outcome, Error> {
        match self.gift_note {
            Some(_) => Self::succeed(),
            None => Self::skip(),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = WorkflowDescriptor::builder("orders")
        .step(Ship::descriptor(step_name!("ship")))
        .build()?;
    let instance = orders
        .instance(Orders)
        .data("gift_note", None::<String>)
        .create()?;
    LocalExecutor::new().run(instance)?;
    Ok(())
}
