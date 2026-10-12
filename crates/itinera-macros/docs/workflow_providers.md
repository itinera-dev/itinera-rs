
# Service providers

A method of the workflow's `impl` block marked `#[provider]` is one of its service providers, which supplies the steps and rescue steps the service they request with `#[service]`. It takes `&self` and returns `Result<Box<T>, Error>`, where `T` is the trait object the steps request, such as `dyn PaymentGateway`. Its attribute takes, each optional:

1. `per_attempt`: the provider is called for every attempt of every step it serves, and its instance is dropped when the attempt ends. Without it, it is of journey scope: called the first time a step it serves needs the service, and its instance is lent to every later attempt until the journey ends;
2. `steps = ["a", "b"]`: the steps it is attached to, which it serves for its service. A rescue step is named as a step, such as `"charge_rescue"` for the rescue step of `charge`, and a provider attached to the step it rescues never serves it. Without it, the provider is attached to no step, and serves every step and rescue step that requests the service and has no provider of its own for it;
3. `key = "..."`: the key of the service it supplies, which is otherwise the method's name;
4. `name = "..."`: its name, which is otherwise the method's name.

Its parameters request what it reads, each marked with its attribute: `#[journey_id]` with `&JourneyId`, `#[from_workflow]` in the forms an input adapter takes, and `#[reporter]` with a `HookReporter<'_>`, or an `AsyncHookReporter<'_>` in an asynchronous workflow. A provider of attempt scope may also request `#[step_name]` with `StepName` and `#[attempt]` with `NonZeroU32`; one of journey scope may not, since it serves whichever step needs its service first. An `async fn` provider belongs only to an asynchronous workflow; a synchronous one serves a workflow of either mode, unless it requests a reporter, whose type belongs to one mode.

Mistakes in the providers are compile errors too: a service a step or rescue step requests that no provider serves, two providers of one service attached to no step or to one step, two providers of one name, a provider attached to a step the workflow does not list, counting the names of rescue steps, or listing a step twice, an empty `steps = []`, a request the provider's scope does not allow, a provider of the other mode, and a service a step or rescue step requests as another type than its providers make. Only when the providers of one key box types written differently in their return types, or write one other than as `Result<Box<T>, ...>`, are the steps and rescue steps they serve left to the descriptor: one requesting another type than its provider makes then panics the first call to `descriptor()`, with the violation `wrong service type`.

```
use std::num::NonZeroU32;

use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, InitialData, JourneyId, JourneyStatus};
use itinera::step::{Outcome, Step, StepName, step};
use itinera::workflow::{Workflow, workflow};

trait PaymentGateway: Send + Sync {
    fn charge(&self, amount: i64) -> String;
}

trait StockStore: Send + Sync {
    fn reserve(&self) -> bool;
}

struct Terminal {
    shop: &'static str,
    attempt: NonZeroU32,
}

impl PaymentGateway for Terminal {
    fn charge(&self, amount: i64) -> String {
        format!("{amount} paid at {}, attempt {}", self.shop, self.attempt)
    }
}

struct Warehouse;

impl StockStore for Warehouse {
    fn reserve(&self) -> bool {
        true
    }
}

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
        #[service] stock: &dyn StockStore,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        if !stock.reserve() {
            return Self::fail("out-of-stock");
        }
        contributor.contribute("receipt", payments.charge(self.amount));
        Self::succeed()
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", retry_budget = 1)]
struct Orders {
    shop: &'static str,
}

#[workflow]
impl Orders {
    #[provider]
    fn stock(&self, #[journey_id] _journey_id: &JourneyId) -> Result<Box<dyn StockStore>, Error> {
        Ok(Box::new(Warehouse))
    }

    #[provider(per_attempt, steps = ["charge"])]
    fn payments(
        &self,
        #[step_name] _step: StepName,
        #[attempt] attempt: NonZeroU32,
    ) -> Result<Box<dyn PaymentGateway>, Error> {
        Ok(Box::new(Terminal {
            shop: self.shop,
            attempt,
        }))
    }
}

#[derive(InitialData)]
struct NewOrder {
    amount: i64,
}

let listing = Orders::descriptor().listing();
assert_eq!(listing.first().map(|step| step.services.len()), Some(2));
let instance = Orders::builder()
    .shop("corner")
    .initial_data(NewOrder { amount: 40 })
    .create()?;
let result = LocalExecutor::new().run(instance)?;
assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
# Ok::<(), Box<dyn std::error::Error>>(())
```

A service that no provider serves does not compile:

```compile_fail
use itinera::error::Error;
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

trait PaymentGateway: Send + Sync {}

#[derive(Step)]
struct Charge;

#[step]
impl Charge {
    fn run(self, #[service] payments: &dyn PaymentGateway) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

#[workflow]
#[with_step(Charge, name = "charge")]
struct Orders;
```
