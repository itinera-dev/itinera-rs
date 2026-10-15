//! The payment workflow: charges an order's price, less its discount, then ships it, once it is
//! packed when it is fragile.

use itinera::error::Error;
use itinera::instance::{Instance, InstanceError};
use itinera::journey::{Contributor, DataBag, InitialData, JourneyId};
use itinera::policy::{OnSuccess, step_policy, workflow_policy};
use itinera::step::{Outcome, Reason, RescueOutcome, RescueStep, Step, rescue_step, step};
use itinera::workflow::{Supplied, role, workflow};

/// Charges the amount an input adapter supplies.
#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    fn run(self, #[contributor] contributor: &mut Contributor) -> Result<Outcome, Error> {
        if self.amount <= 0 {
            return Self::fail("declined");
        }
        contributor.contribute("charged", self.amount);
        Self::succeed()
    }
}

/// Ships the order, unless it is fragile and not packed.
#[derive(Step)]
struct Ship {
    #[input]
    fragile: Option<bool>,
    #[input]
    packed: Option<bool>,
}

#[step]
impl Ship {
    fn run(self, #[contributor] contributor: &mut Contributor) -> Result<Outcome, Error> {
        if self.fragile == Some(true) && self.packed.is_none() {
            return Self::fail("unpacked");
        }
        contributor.contribute("shipped", true);
        Self::succeed()
    }
}

/// Packs an order that could not be shipped, saying why.
#[derive(RescueStep)]
struct Pack {
    #[input]
    order: String,
    #[reason]
    reason: Reason,
}

#[rescue_step]
impl Pack {
    fn run(self, #[contributor] contributor: &mut Contributor) -> Result<RescueOutcome, Error> {
        let packing = format!(
            "order {} packed, since it was {}",
            self.order,
            self.reason.code()
        );
        contributor.contribute("packed", true);
        contributor.contribute("packing", packing);
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
        #[contributor] contributor: &mut Contributor,
    ) -> Result<OnSuccess, Error> {
        contributor.contribute("audited", format!("charged {charged}"));
        Self::keep_default()
    }
}

/// A role of the payment workflow: the shop its receipts name.
trait Shop {
    fn shop(&self) -> &'static str;
}

/// Gives a receipt for a paid order, and notes a declined one.
#[derive(Default)]
struct Receipt;

#[workflow_policy(name = "receipt")]
impl Receipt {
    #[on_workflow_success]
    fn paid(
        &self,
        #[journey_id] journey_id: &JourneyId,
        #[from_workflow] charged: i64,
        #[role] shop: &dyn Shop,
        #[contributor] contributor: &mut Contributor,
    ) -> Result<(), Error> {
        let receipt = format!("{journey_id} paid {charged} at the {} shop", shop.shop());
        contributor.contribute("receipt", receipt);
        Ok(())
    }

    #[on_workflow_failure]
    fn declined(&self, #[contributor] contributor: &mut Contributor) -> Result<(), Error> {
        contributor.contribute("receipt", "declined".to_owned());
        Ok(())
    }
}

#[workflow(name = "payment", policies = [Receipt])]
#[with_step(Charge, name = "charge", retry_budget = 1, policies = [Audit])]
#[with_step(Ship, name = "ship", rescue = Pack)]
pub(super) struct Payment {
    shop: &'static str,
}

#[role]
impl Shop for Payment {
    fn shop(&self) -> &'static str {
        self.shop
    }
}

#[workflow]
impl Payment {
    #[input_adapter(steps = ["charge"])]
    fn pricing(
        &self,
        #[from_workflow] price: Option<i64>,
        #[from_workflow] discount: Option<i64>,
    ) -> Result<Supplied, Error> {
        match price {
            Some(price) => Self::supply(less(price, discount)),
            None => Self::from_data_bag(),
        }
    }

    #[id_generator]
    fn order_id(&self, data: &DataBag) -> Result<String, Error> {
        let order: String = data.required("order")?;
        Ok(format!("{}-{order}", self.shop))
    }
}

fn less(price: i64, discount: Option<i64>) -> i64 {
    price - discount.unwrap_or_default()
}

/// An instance of the payment workflow, for the shop "corner".
pub(super) fn instance(
    data: impl InitialData + 'static,
) -> Result<Instance<Payment>, InstanceError> {
    Payment::builder()
        .shop("corner")
        .initial_data(data)
        .create()
}
