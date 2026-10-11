Makes the workflow provide a role, on its implementation of the role's trait, so that hooks may request the role with `#[role]`.

The attribute takes no arguments. It keeps the implementation as written and adds `Provides<dyn Role>` for the workflow, which hands the workflow itself to every hook that requests the role, as `&dyn Role`. The role's trait is therefore dyn compatible.

# Examples

A step policy that prints receipts at the till of the workflow running it:

```
use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, JourneyStatus};
use itinera::policy::{OnSuccess, step_policy};
use itinera::step::{Outcome, Step, step};
use itinera::workflow::{role, workflow};

trait Till {
    fn till(&self) -> &'static str;
}

#[derive(Default)]
struct Receipt;

#[step_policy(name = "receipt")]
impl Receipt {
    #[on_step_success]
    fn printed(
        &self,
        #[role] till: &dyn Till,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Option<OnSuccess>, Error> {
        contributor.contribute("receipt", format!("printed at the {}", till.till()));
        Self::carry_on()
    }
}

#[derive(Step)]
struct Charge;

#[step]
impl Charge {
    fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

#[workflow(name = "shop")]
#[with_step(Charge, name = "charge", policies = [Receipt])]
struct Shop;

#[role]
impl Till for Shop {
    fn till(&self) -> &'static str {
        "front till"
    }
}

let result = LocalExecutor::new().run(Shop::builder().create()?)?;
assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
# Ok::<(), Box<dyn std::error::Error>>(())
```
