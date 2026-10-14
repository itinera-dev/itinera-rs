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
struct Orders;

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut executor = local_executor_for!(Orders);
    for _ in 0..2 {
        let result = executor.run(Orders::builder().create()?)?;
        assert_eq!(result.status.kind(), StatusKind::Succeeded);
    }
    Ok(())
}
