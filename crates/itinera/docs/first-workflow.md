# A first workflow

A workflow of one step, which charges an order's price, with a step policy that audits each
charge. The order is given as the journey's initial data, and the receipt is read from its
output.

```
use itinera::error::Result;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, InitialData, Output};
use itinera::policy::{OnSuccess, step_policy};
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

#[derive(Step)]
struct Charge {
    #[input]
    price: i64,
}

#[step]
impl Charge {
    fn run(self, #[contributor] mut contributor: Contributor<'_>) -> Result<Outcome> {
        if self.price <= 0 {
            return Self::fail("nothing-to-charge");
        }
        contributor.contribute("charged", self.price);
        Self::succeed()
    }
}

#[derive(Default)]
struct Audit;

#[step_policy(name = "audit")]
impl Audit {
    #[on_step_success]
    fn audited(
        &self,
        #[from_step] charged: i64,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Option<OnSuccess>> {
        contributor.contribute("audit", format!("charged {charged}"));
        Self::carry_on()
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", retry_budget = 2, policies = [Audit])]
struct Orders;

#[derive(InitialData)]
struct NewOrder {
    price: i64,
}

#[derive(Output)]
struct Receipt {
    charged: i64,
    audit: String,
}

let instance = Orders::builder()
    .initial_data(NewOrder { price: 40 })
    .create()?;
let receipt: Receipt = LocalExecutor::new().run(instance)?.output()?;
assert_eq!(receipt.charged, 40);
assert_eq!(receipt.audit, "charged 40");
# Ok::<(), Box<dyn std::error::Error>>(())
```

The crate's examples go further: `orders` adds a second step, a workflow policy using a role,
an input adapter, an ID generator and a reporter; `orders_async` runs the same workflow on the
asynchronous executor; and `orders_by_hand` declares it with the builder alone, without macros.
