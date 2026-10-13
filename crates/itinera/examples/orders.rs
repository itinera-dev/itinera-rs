//! A synchronous workflow declared with the macros: it charges an order, less its discount,
//! through the shop's card terminal, then ships it, and tells the customer once the order is paid
//! and shipped. Run it with `cargo run -p itinera --example orders`.

use std::num::NonZeroU32;

use itinera::error::Error;
use itinera::event::Event;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, DataBag, InitialData, JourneyId};
use itinera::policy::{OnSuccess, step_policy, workflow_policy};
use itinera::report::Reporter;
use itinera::service::{Provided, provide};
use itinera::step::{Outcome, Step, step};
use itinera::value::AnyValue;
use itinera::workflow::{role, workflow};

/// Takes payments: the service a charge is paid through.
trait PaymentGateway: Send + Sync {
    /// Takes the amount, and says how it was paid.
    fn take(&self, amount: i64) -> String;
}

/// The shop's card terminal, lent to one attempt to charge.
struct Terminal {
    shop: &'static str,
    attempt: NonZeroU32,
}

impl PaymentGateway for Terminal {
    fn take(&self, amount: i64) -> String {
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
    fn run(
        self,
        #[service] payments: &dyn PaymentGateway,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        if self.amount <= 0 {
            return Self::fail_retriably("declined");
        }
        contributor.contribute("payment", payments.take(self.amount));
        contributor.contribute("charged", self.amount);
        Self::succeed()
    }
}

/// Ships the order.
#[derive(Step)]
struct Ship;

#[step]
impl Ship {
    fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

/// Notes each amount charged.
#[derive(Default)]
struct Audit;

#[step_policy(name = "audit")]
impl Audit {
    #[on_step_success]
    fn audited(
        &self,
        #[from_step] charged: i64,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Option<OnSuccess>, Error> {
        contributor.contribute("audit", format!("charged {charged}"));
        Self::carry_on()
    }
}

/// A role the shop provides: telling a customer something.
trait Messages {
    fn tell(&self, message: &str);
}

/// Tells the customer once the order is paid and shipped.
#[derive(Default)]
struct Notify;

#[workflow_policy(name = "notify")]
impl Notify {
    #[on_workflow_success]
    fn paid(
        &self,
        #[journey_id] journey_id: &JourneyId,
        #[from_workflow] charged: i64,
        #[from_workflow] payment: String,
        #[role] messages: &dyn Messages,
    ) -> Result<(), Error> {
        messages.tell(&format!("order {journey_id}: paid {charged}, {payment}"));
        Ok(())
    }
}

/// Prints each event of every journey.
#[derive(Default)]
struct Printer;

impl Reporter for Printer {
    fn report(&mut self, event: &Event) -> Result<(), Error> {
        println!("{} {}", event.journey_id, event.kind());
        Ok(())
    }
}

#[workflow(name = "orders", policies = [Notify], reporters = [Printer])]
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
        Self::supply(price - discount.unwrap_or_default())
    }

    /// Lends each attempt to charge the shop's card terminal.
    #[provider(per_attempt, steps = ["charge"])]
    fn payments(
        &self,
        #[attempt] attempt: NonZeroU32,
    ) -> Result<Provided<dyn PaymentGateway>, Error> {
        Ok(provide!(Terminal {
            shop: self.shop,
            attempt,
        }))
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
    let mut executor = LocalExecutor::new();
    for order in orders {
        let instance = Orders::builder()
            .shop("corner")
            .initial_data(order)
            .create()?;
        let result = executor.run(instance)?;
        println!("{}: {}", result.journey_id, result.status.kind());
    }
    Ok(())
}
