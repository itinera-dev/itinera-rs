use itinera::error::Error;
use itinera::executor::AsyncLocalExecutor;
use itinera::journey::Contributor;
use itinera::policy::AsyncHookReporter;
use itinera::service::{Provided, provide};
use itinera::step::{Attempt, Outcome, step};
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
    fn new(#[input] amount: i64) -> Self {
        Self { amount }
    }

    async fn run(
        self,
        #[service] payments: &dyn PaymentGateway,
        #[service] stock: &dyn StockStore,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        stock.reserve();
        contributor.contribute("receipt", payments.charge(self.amount));
        Self::succeed()
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", retry_budget = 1)]
struct Orders {
    gateway: &'static str,
}

#[workflow]
impl Orders {
    #[provider]
    fn stock(
        &self,
        #[reporter] _reporter: itinera::policy::HookReporter<'_>,
    ) -> Result<Provided<dyn StockStore>, Error> {
        Ok(provide!(Warehouse))
    }

    #[provider(per_attempt)]
    async fn payments(
        &self,
        #[attempt] attempt: Attempt,
        #[reporter] mut reporter: AsyncHookReporter<'_>,
    ) -> Result<Provided<dyn PaymentGateway>, Error> {
        reporter.info(format!("attempt {attempt}")).await?;
        Ok(provide!(Gateway(self.gateway.to_owned())))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let instance = Orders::builder()
        .gateway("main")
        .initial_data(NewOrder { amount: 40 })
        .create()?;
    let result = futures::executor::block_on(AsyncLocalExecutor::new().run(instance))?;
    assert!(result.status.is_succeeded());
    Ok(())
}

#[derive(itinera::journey::InitialData)]
struct NewOrder {
    amount: i64,
}
