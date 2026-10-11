//! The warehouse workflow: reserves an order's goods, charges it and ships it, through the stock
//! and the payments its service providers supply.

use std::num::NonZeroU32;

use itinera::error::Error;
use itinera::instance::{Instance, InstanceError};
use itinera::journey::{Contributor, DataBag, InitialData, JourneyId};
use itinera::policy::HookReporter;
use itinera::step::{Outcome, Step, StepName, step};
use itinera::workflow::workflow;

/// The goods of a journey, which every step that needs them shares.
trait Stock: Send + Sync {
    fn label(&self) -> &str;
}

/// Takes payments, unless its gateway is unreachable.
trait Payments: Send + Sync {
    fn label(&self) -> &str;

    fn reachable(&self) -> bool;
}

struct Shelf {
    label: String,
}

impl Stock for Shelf {
    fn label(&self) -> &str {
        &self.label
    }
}

struct Gateway {
    label: String,
    reachable: bool,
}

impl Payments for Gateway {
    fn label(&self) -> &str {
        &self.label
    }

    fn reachable(&self) -> bool {
        self.reachable
    }
}

/// Reserves the order's goods.
#[derive(Step)]
struct Reserve;

#[step]
impl Reserve {
    fn run(
        self,
        #[service] stock: &dyn Stock,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        contributor.contribute("reserved", stock.label().to_owned());
        Self::succeed()
    }
}

/// Charges the order, which may be retried while the gateway is unreachable.
#[derive(Step)]
struct Charge;

#[step]
impl Charge {
    fn run(
        self,
        #[service] payments: &dyn Payments,
        #[service] stock: &dyn Stock,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        if !payments.reachable() {
            return Self::fail_retriably("unreachable");
        }
        contributor.contribute("charged", used(payments, stock));
        Self::succeed()
    }
}

/// Ships the order.
#[derive(Step)]
struct Ship;

#[step]
impl Ship {
    fn run(
        self,
        #[service] payments: &dyn Payments,
        #[service] stock: &dyn Stock,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        contributor.contribute("shipped", used(payments, stock));
        Self::succeed()
    }
}

/// The labels of the services a step used.
fn used(payments: &dyn Payments, stock: &dyn Stock) -> String {
    format!("{} with {}", payments.label(), stock.label())
}

#[workflow(name = "warehouse")]
#[with_step(Reserve, name = "reserve")]
#[with_step(Charge, name = "charge", retry_budget = 1)]
#[with_step(Ship, name = "ship")]
pub(super) struct Warehouse;

#[workflow]
impl Warehouse {
    /// The journey's stock, made once and lent to every step and attempt.
    #[provider]
    fn stock(&self, #[journey_id] journey_id: &JourneyId) -> Result<Box<dyn Stock>, Error> {
        Ok(Box::new(Shelf {
            label: format!("stock of {journey_id}"),
        }))
    }

    /// The payments of every step without a provider of its own for them.
    #[provider]
    fn payments(&self) -> Result<Box<dyn Payments>, Error> {
        Ok(Box::new(Gateway {
            label: "payments".to_owned(),
            reachable: true,
        }))
    }

    /// Fresh payments for each attempt to charge, through the gateway the data bag names: the
    /// main one unless it names another. A flaky gateway is unreachable on a first attempt, and
    /// one that is down cannot be made.
    #[provider(per_attempt, steps = ["charge"], key = "payments", name = "fresh payments")]
    fn fresh_payments(
        &self,
        #[step_name] step: StepName,
        #[attempt] attempt: NonZeroU32,
        #[from_workflow] gateway: Option<String>,
        #[reporter] reporter: HookReporter<'_>,
    ) -> Result<Box<dyn Payments>, Error> {
        let gateway = gateway.unwrap_or_else(|| "main".to_owned());
        chosen(&gateway, step, attempt, reporter)
    }

    #[id_generator]
    fn order_id(&self, data: &DataBag) -> Result<String, Error> {
        let order: String = data.required("order")?;
        Ok(format!("order-{order}"))
    }
}

fn chosen(
    gateway: &str,
    step: StepName,
    attempt: NonZeroU32,
    mut reporter: HookReporter<'_>,
) -> Result<Box<dyn Payments>, Error> {
    if gateway == "down" {
        return Err(Error::msg("gateway down"));
    }
    reporter.info(format!("choosing the {gateway} gateway"))?;
    Ok(Box::new(Gateway {
        label: format!("{gateway} gateway for {step}, attempt {attempt}"),
        reachable: gateway != "flaky" || attempt.get() > 1,
    }))
}

/// An instance of the warehouse workflow.
pub(super) fn instance(
    data: impl InitialData + 'static,
) -> Result<Instance<Warehouse>, InstanceError> {
    Warehouse::builder().initial_data(data).create()
}
