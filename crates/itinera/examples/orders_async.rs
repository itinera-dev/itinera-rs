//! The workflow of the `orders` example, declared as an asynchronous workflow: its steps, its
//! policies and its reporter are asynchronous, and it runs on the asynchronous executor. Run it
//! with `cargo run -p itinera --example orders_async --features async`.

use itinera::error::Error;
use itinera::event::Event;
use itinera::executor::AsyncLocalExecutor;
use itinera::journey::{Contributor, DataBag, InitialData, JourneyId};
use itinera::policy::{OnSuccess, step_policy, workflow_policy};
use itinera::report::AsyncReporter;
use itinera::step::{Outcome, Step, step};
use itinera::value::AnyValue;
use itinera::workflow::{role, workflow};

/// Charges the amount the workflow's pricing works out. A charge of nothing is declined, which
/// may be retried.
#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    async fn run(self, #[contributor] mut contributor: Contributor<'_>) -> Result<Outcome, Error> {
        if self.amount <= 0 {
            return Self::fail_retriably("declined");
        }
        contributor.contribute("charged", self.amount);
        Self::succeed()
    }
}

/// Ships the order.
#[derive(Step)]
struct Ship;

#[step]
impl Ship {
    async fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

/// Notes each amount charged.
#[derive(Default)]
struct Audit;

#[step_policy(name = "audit")]
impl Audit {
    #[on_step_success]
    async fn audited(
        &self,
        #[from_step] charged: i64,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Option<OnSuccess>, Error> {
        contributor.contribute("audit", format!("charged {charged}"));
        Self::carry_on()
    }
}

/// A role the shop provides: telling a customer something. An asynchronous hook's future may move
/// between threads while it holds the role, so the role is `Sync`.
trait Messages: Sync {
    fn tell(&self, message: &str);
}

/// Tells the customer once the order is paid and shipped.
#[derive(Default)]
struct Notify;

#[workflow_policy(name = "notify")]
impl Notify {
    #[on_workflow_success]
    async fn paid(
        &self,
        #[journey_id] journey_id: &JourneyId,
        #[from_workflow] charged: i64,
        #[role] messages: &dyn Messages,
    ) -> Result<(), Error> {
        messages.tell(&format!("order {journey_id}: paid {charged}"));
        Ok(())
    }
}

/// Prints each event of every journey.
#[derive(Default)]
struct Printer;

impl AsyncReporter for Printer {
    async fn report(&mut self, event: &Event) -> Result<(), Error> {
        println!("{} {}", event.journey_id, event.kind());
        Ok(())
    }
}

#[workflow(name = "orders", asynchronous, policies = [Notify], async_reporters = [Printer])]
#[with_step(Charge, name = "charge", retry_budget = 1, policies = [Audit])]
#[with_step(Ship, name = "ship")]
struct Orders {
    shop: &'static str,
}

#[role]
impl Messages for Orders {
    fn tell(&self, message: &str) {
        println!("{}: {message}", self.shop);
    }
}

#[workflow]
impl Orders {
    /// Works out the amount to charge: the price, less any discount.
    #[input_adapter(steps = ["charge"])]
    fn pricing(
        &self,
        #[from_workflow] price: i64,
        #[from_workflow] discount: Option<i64>,
    ) -> Result<Option<AnyValue>, Error> {
        Ok(Some(AnyValue::new(price - discount.unwrap_or_default())))
    }

    /// Names each journey after the shop and the order.
    #[id_generator]
    fn order_id(&self, data: &DataBag) -> Result<String, Error> {
        let order: String = data.required("order")?;
        Ok(format!("{}-{order}", self.shop))
    }
}

#[derive(InitialData)]
struct NewOrder {
    order: String,
    price: i64,
    discount: Option<i64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orders = [
        NewOrder {
            order: "1".to_owned(),
            price: 40,
            discount: Some(5),
        },
        NewOrder {
            order: "2".to_owned(),
            price: 10,
            discount: Some(10),
        },
    ];
    let mut executor = AsyncLocalExecutor::new();
    for order in orders {
        let instance = Orders::builder()
            .shop("corner")
            .initial_data(order)
            .create()?;
        let result = futures::executor::block_on(executor.run(instance))?;
        println!("{}: {}", result.journey_id, result.status.kind());
    }
    Ok(())
}
