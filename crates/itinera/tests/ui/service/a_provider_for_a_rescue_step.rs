use std::num::NonZeroU32;

use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, JourneyId, JourneyStatus};
use itinera::step::{Outcome, RescueOutcome, RescueStep, Step, rescue_step, step};
use itinera::workflow::workflow;

trait TokenService: Send + Sync {
    fn token(&self, account: &str) -> String;
}

struct Vault(String);

impl TokenService for Vault {
    fn token(&self, account: &str) -> String {
        format!("{account} from {}", self.0)
    }
}

#[derive(Step)]
struct Charge {
    #[input]
    token: Option<String>,
}

#[step]
impl Charge {
    fn run(self, #[service] tokens: &dyn TokenService) -> Result<Outcome, Error> {
        match self.token {
            Some(token) if token.ends_with(&tokens.token("")) => Self::fail("wrong-vault"),
            Some(_) => Self::succeed(),
            None => Self::fail("token-expired"),
        }
    }
}

#[derive(RescueStep)]
struct Refresh {
    #[input]
    account: String,
}

#[rescue_step]
impl Refresh {
    fn run(
        self,
        #[service] tokens: &dyn TokenService,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<RescueOutcome, Error> {
        contributor.contribute("token", tokens.token(&self.account));
        Self::succeed()
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", rescue = Refresh)]
struct Orders;

#[workflow]
impl Orders {
    #[provider]
    fn tokens(&self, #[journey_id] journey: &JourneyId) -> Result<Box<dyn TokenService>, Error> {
        Ok(Box::new(Vault(format!("the vault of {journey}"))))
    }

    #[provider(per_attempt, steps = ["charge", "charge_rescue"], key = "tokens", name = "charging")]
    fn charging(&self, #[attempt] attempt: NonZeroU32) -> Result<Box<dyn TokenService>, Error> {
        Ok(Box::new(Vault(format!("the vault of attempt {attempt}"))))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Orders::builder()
        .initial_data(Client { account: "A-7".to_owned() })
        .create()?;
    let result = LocalExecutor::new().run(instance)?;
    assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    Ok(())
}

#[derive(itinera::journey::InitialData)]
struct Client {
    account: String,
}
