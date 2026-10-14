use itinera::error::Error;
use itinera::executor::local_executor_for;
use itinera::journey::StatusKind;
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

#[derive(Step)]
struct Charge;

#[step]
impl Charge {
    fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge")]
struct Orders {
    shop: &'static str,
}

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut executor = local_executor_for!(Orders);
    for shop in ["corner", "market"] {
        let result = executor.run(Orders::builder().shop(shop).create()?)?;
        assert_eq!(result.status.kind(), StatusKind::Succeeded);
    }
    Ok(())
}
