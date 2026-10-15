use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, JourneyId, JourneyStatus};
use itinera::service::{Provided, provide};
use itinera::step::{Attempt, Outcome, RescueOutcome, RescueStep, Step, rescue_step, step};
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
        contributor: &mut Contributor,
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
    fn tokens(&self, journey: &JourneyId) -> Result<Provided<dyn TokenService>, Error> {
        Ok(provide!(Vault(format!("the vault of {journey}"))))
    }

    #[provider(per_attempt, steps = ["charge", "charge_rescue"], key = "tokens", name = "charging")]
    fn charging(&self, attempt: Attempt) -> Result<Provided<dyn TokenService>, Error> {
        Ok(provide!(Vault(format!("the vault of attempt {attempt}"))))
    }
}

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Orders::builder()
        .initial_data(Client {
            account: "A-7".to_owned(),
        })
        .create()?;
    let result = LocalExecutor::new().run(instance)?;
    let JourneyStatus::Succeeded { data, .. } = result.status else {
        return Err("the rescue step recovers the journey".into());
    };
    assert_eq!(
        data.required::<String>("token")?,
        "A-7 from the vault of attempt 1"
    );
    Ok(())
}

#[derive(itinera::journey::InitialData)]
struct Client {
    account: String,
}
