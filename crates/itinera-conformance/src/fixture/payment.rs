//! The payment workflow: charges an order's price, less its discount, then ships it.

use itinera::error::Error;
use itinera::instance::{Instance, InstanceError};
use itinera::journey::{Contributor, DataBag, InitialData, JourneyId};
use itinera::policy::{OnSuccess, step_policy, workflow_policy};
use itinera::step::{Outcome, Step, step};
use itinera::value::AnyValue;
use itinera::workflow::{role, workflow};

/// Charges the amount an input adapter supplies.
#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    fn run(self, #[contributor] mut contributor: Contributor<'_>) -> Result<Outcome, Error> {
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
    fn run(self, #[contributor] mut contributor: Contributor<'_>) -> Result<Outcome, Error> {
        contributor.contribute("shipped", true);
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
        contributor.contribute("audited", format!("charged {charged}"));
        Self::carry_on()
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
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<(), Error> {
        let receipt = format!("{journey_id} paid {charged} at the {} shop", shop.shop());
        contributor.contribute("receipt", receipt);
        Ok(())
    }

    #[on_workflow_failure]
    fn declined(&self, #[contributor] mut contributor: Contributor<'_>) -> Result<(), Error> {
        contributor.contribute("receipt", "declined".to_owned());
        Ok(())
    }
}

#[workflow(name = "payment", policies = [Receipt])]
#[with_step(Charge, name = "charge", retry_budget = 1, policies = [Audit])]
#[with_step(Ship, name = "ship")]
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
    ) -> Result<Option<AnyValue>, Error> {
        Ok(price.map(|price| less(price, discount)).map(AnyValue::new))
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
