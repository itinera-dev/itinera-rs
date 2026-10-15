use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::Contributor;
use itinera::step::{Outcome, Reason, RescueOutcome, RescueStep, Step, rescue_step, step};
use itinera::workflow::workflow;

#[derive(Step)]
struct Charge {
    #[input]
    token: Option<String>,
}

#[step]
impl Charge {
    fn run(self) -> Result<Outcome, Error> {
        match self.token {
            Some(_) => Self::succeed(),
            None => Self::fail("token-expired"),
        }
    }
}

#[derive(RescueStep)]
struct Refresh {
    #[input]
    client: String,
    #[reason]
    reason: Reason,
}

#[rescue_step]
impl Refresh {
    async fn run(
        self,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<RescueOutcome, Error> {
        contributor.contribute("token", format!("{}-{}", self.client, self.reason.code()));
        Self::succeed()
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", rescue = Refresh)]
#[with_step(Charge, name = "charge again")]
struct Orders;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Orders::builder().initial_data(Client { client: "C-1".to_owned() }).create()?;
    let result = LocalExecutor::new().run(instance)?;
    assert!(result.status.is_succeeded());
    Ok(())
}

#[derive(itinera::journey::InitialData)]
struct Client {
    client: String,
}
