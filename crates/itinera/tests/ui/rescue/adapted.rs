use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, JourneyStatus};
use itinera::step::{Outcome, Reason, RescueOutcome, RescueStep, Step, rescue_step, step};
use itinera::workflow::{Supplied, workflow};

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
    reason: Reason,
}

#[rescue_step]
impl Refresh {
    fn run(self, contributor: &mut Contributor) -> Result<RescueOutcome, Error> {
        contributor.contribute("token", format!("{}-{}", self.client, self.reason.code()));
        Self::succeed()
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", rescue = Refresh)]
#[with_step(Charge, name = "charge again")]
struct Orders;

#[workflow]
impl Orders {
    #[input_adapter(steps = ["charge_rescue"])]
    fn client(&self) -> Result<Supplied, Error> {
        Self::supply("C-2".to_owned())
    }
}

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Orders::builder()
        .initial_data(Client {
            client: "C-1".to_owned(),
        })
        .create()?;
    let result = LocalExecutor::new().run(instance)?;
    let JourneyStatus::Succeeded { data, .. } = result.status else {
        return Err("the rescue step recovers the journey".into());
    };
    assert_eq!(data.required::<String>("token")?, "C-2-token-expired");
    Ok(())
}

#[derive(itinera::journey::InitialData)]
struct Client {
    client: String,
}
