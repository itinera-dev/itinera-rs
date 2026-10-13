use std::num::NonZeroU32;

use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, JourneyId, JourneyStatus};
use itinera::policy::HookReporter;
use itinera::service::{Provided, provide};
use itinera::step::{Outcome, Step, StepName, step};
use itinera::workflow::workflow;

trait PaymentGateway: Send + Sync {
    fn charge(&self, amount: i64) -> String;
}

trait StockStore: Send + Sync {
    fn reserve(&self) -> bool;
}

struct Gateway(String);

impl PaymentGateway for Gateway {
    fn charge(&self, amount: i64) -> String {
        format!("{amount} through {}", self.0)
    }
}

struct Warehouse;

impl StockStore for Warehouse {
    fn reserve(&self) -> bool {
        true
    }
}

struct Charge {
    amount: i64,
}

#[step]
impl Charge {
    fn new(#[input] amount: i64, #[service] stock: &dyn StockStore) -> Result<Self, Error> {
        if stock.reserve() {
            Ok(Self { amount })
        } else {
            Err(Error::msg("out of stock"))
        }
    }

    fn run(
        self,
        #[service("payments")] gateway: &dyn PaymentGateway,
        #[service] stock: &dyn StockStore,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        stock.reserve();
        contributor.contribute("receipt", gateway.charge(self.amount));
        Self::succeed()
    }
}

#[derive(Step)]
struct Ship;

#[step]
impl Ship {
    fn run(self, #[service] stock: &dyn StockStore) -> Result<Outcome, Error> {
        if stock.reserve() {
            Self::succeed()
        } else {
            Self::fail("out-of-stock")
        }
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", retry_budget = 1)]
#[with_step(Ship, name = "ship")]
struct Orders {
    gateway: &'static str,
}

#[workflow]
impl Orders {
    #[provider]
    fn stock(&self, #[journey_id] _journey: &JourneyId) -> Result<Provided<dyn StockStore>, Error> {
        Ok(provide!(Warehouse))
    }

    #[provider(per_attempt, steps = ["charge"], key = "payments", name = "charging")]
    fn payments(
        &self,
        #[step_name] step: StepName,
        #[attempt] attempt: NonZeroU32,
        #[from_workflow] currency: Option<String>,
        #[reporter] mut reporter: HookReporter<'_>,
    ) -> Result<Provided<dyn PaymentGateway>, Error> {
        reporter.info(format!("{step} attempt {attempt}"))?;
        let currency = currency.unwrap_or_default();
        Ok(provide!(Gateway(format!("{} {currency}", self.gateway))))
    }
}

pub(crate) fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Orders::builder()
        .gateway("main")
        .initial_data(NewOrder { amount: 40 })
        .create()?;
    let result = LocalExecutor::new().run(instance)?;
    assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    Ok(())
}

#[derive(itinera::journey::InitialData)]
struct NewOrder {
    amount: i64,
}
