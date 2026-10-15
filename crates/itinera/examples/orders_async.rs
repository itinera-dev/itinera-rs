//! The workflow of the `orders` example, declared as an asynchronous workflow: its steps, its
//! policies, its service and its reporter are asynchronous, and it runs on the asynchronous
//! executor. Run it with `cargo run -p itinera --example orders_async --features async`.

use std::num::NonZeroU32;

use itinera::error::Result;
use itinera::event::Event;
use itinera::executor::local_executor_for;
use itinera::journey::{Contributor, DataBag, InitialData, JourneyId};
use itinera::mode::async_trait;
use itinera::policy::{OnSuccess, step_policy, workflow_policy};
use itinera::report::AsyncReporter;
use itinera::service::{Provided, provide};
use itinera::step::{Outcome, Step, step};
use itinera::value::AnyValue;
use itinera::workflow::{role, workflow};

/// Takes payments, asynchronously: the service a charge is paid through.
#[async_trait]
trait PaymentGateway: Send + Sync {
    /// Takes the amount, and says how it was paid.
    async fn take(&self, amount: i64) -> String;
}

/// The shop's card terminal, lent to one attempt to charge.
struct Terminal {
    shop: &'static str,
    attempt: NonZeroU32,
}

#[async_trait]
impl PaymentGateway for Terminal {
    async fn take(&self, amount: i64) -> String {
        format!(
            "{amount} paid at the {} terminal, attempt {}",
            self.shop, self.attempt
        )
    }
}

/// Charges the amount the workflow's pricing works out, through the payment gateway. A charge of
/// nothing is declined, which may be retried.
#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    async fn run(
        self,
        #[service] payments: &dyn PaymentGateway,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome> {
        if self.amount <= 0 {
            return Self::fail("declined");
        }
        contributor.contribute("payment", payments.take(self.amount).await);
        contributor.contribute("charged", self.amount);
        Self::succeed()
    }
}

/// Ships the order.
#[derive(Step)]
struct Ship;

#[step]
impl Ship {
    async fn run(self) -> Result<Outcome> {
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
    ) -> Result<Option<OnSuccess>> {
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
        #[from_workflow] payment: String,
        #[role] messages: &dyn Messages,
    ) -> Result<()> {
        messages.tell(&format!("order {journey_id}: paid {charged}, {payment}"));
        Ok(())
    }
}

/// Prints each event of every journey.
#[derive(Default)]
struct Printer;

impl AsyncReporter for Printer {
    async fn report(&mut self, event: &Event) -> Result<()> {
        println!("{} {}", event.journey_id, event.kind());
        Ok(())
    }
}

#[workflow(name = "orders", policies = [Notify], async_reporters = [Printer])]
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
    ) -> Result<Option<AnyValue>> {
        Self::supply(price - discount.unwrap_or_default())
    }

    /// Lends each attempt to charge the shop's card terminal.
    #[provider(per_attempt, steps = ["charge"])]
    async fn payments(
        &self,
        #[attempt] attempt: NonZeroU32,
    ) -> Result<Provided<dyn PaymentGateway>> {
        Ok(provide!(Terminal {
            shop: self.shop,
            attempt,
        }))
    }

    /// Names each journey after the shop and the order.
    #[id_generator]
    fn order_id(&self, data: &DataBag) -> Result<String> {
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
    let mut executor = local_executor_for!(Orders);
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
