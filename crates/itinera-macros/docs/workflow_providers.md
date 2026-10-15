
# Service providers

A method of the workflow's `impl` block marked `#[provider]` is one of its service providers, which supplies the steps and rescue steps the service they request with `#[service]`. It takes `&self` and returns `Result<Provided<T>, Error>`, where `T` is the trait object the steps request, such as `dyn PaymentGateway`. It makes the `Provided` with `provide!(instance)`, which names the instance's concrete type as the implementation `service_provided` carries, or with `provide!(instance, data)`, which also supplies data for the reporters. An instance that is already a `Box<T>` is made into a `Provided` with `Provided::from_box`, whose implementation can only be `T`: the provider should then say which implementation it made in its data, with `Provided::with_data`. Its attribute takes, each optional:

1. `per_attempt`: the provider is called for every attempt of every step it serves, and its instance is dropped when the attempt ends. Without it, it is of journey scope: called the first time a step it serves needs the service, and its instance is lent to every later attempt until the journey ends;
2. `steps = ["a", "b"]`: the steps it is attached to, which it serves for its service. A rescue step is named as a step, such as `"charge_rescue"` for the rescue step of `charge`, and a provider attached to the step it rescues never serves it. Without it, the provider is attached to no step, and serves every step and rescue step that requests the service and has no provider of its own for it;
3. `key = "..."`: the key of the service it supplies, which is otherwise the method's name;
4. `name = "..."`: its name, which is otherwise the method's name.

Its parameters request what it reads: the journey ID as a `&JourneyId` and a reporter as a `HookReporter`, or, in an `async fn` provider, an `AsyncHookReporter<'_>`, whose lifetime it must write, each recognised by its type as written, by the last segment of its path, so not through an alias, and, marked with its attribute, `#[from_workflow]` in the forms an input adapter takes. A provider of attempt scope may also request the step's name as a `StepName` and the attempt as an `Attempt`; one of journey scope may not, since it serves whichever step needs its service first. An `async fn` provider belongs only to an asynchronous workflow; a synchronous one serves a workflow of either mode, unless it requests a reporter, whose type belongs to one mode.

Mistakes in the providers are compile errors too: a service a step or rescue step requests that no provider serves, two providers of one service attached to no step or to one step, two providers of one name, a provider attached to a step the workflow does not list, counting the names of rescue steps, or listing a step twice, an empty `steps = []`, a request the provider's scope does not allow, a provider of the other mode, and a service a step or rescue step requests as another type than its providers make. Only when the providers of one key provide types written differently in their return types, or write one other than as `Result<Provided<T>, ...>` or `Result<Provided<T>>`, are the steps and rescue steps they serve left to the descriptor: one requesting another type than its provider makes then panics the first call to `descriptor()`, with the violation `wrong service type`.

```
use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, InitialData, JourneyId};
use itinera::service::{Provided, provide};
use itinera::step::{Attempt, Outcome, Step, StepName, step};
use itinera::workflow::{Workflow, workflow};

trait PaymentGateway: Send + Sync {
    fn charge(&self, amount: i64) -> String;
}

trait StockStore: Send + Sync {
    fn reserve(&self) -> bool;
}

struct Terminal {
    shop: &'static str,
    attempt: Attempt,
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
        contributor: &mut Contributor,
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
    fn stock(
        &self,
        _journey_id: &JourneyId,
    ) -> Result<Provided<dyn StockStore>, Error> {
        Ok(provide!(Warehouse))
    }

    #[provider(per_attempt, steps = ["charge"])]
    fn payments(
        &self,
        _step: StepName,
        attempt: Attempt,
    ) -> Result<Provided<dyn PaymentGateway>, Error> {
        let terminal = Terminal {
            shop: self.shop,
            attempt,
        };
        Ok(provide!(terminal, self.shop.to_string()))
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
assert!(result.status.is_succeeded());
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
