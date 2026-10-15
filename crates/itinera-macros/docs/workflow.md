Declares a workflow: on its struct, the workflow's name, steps, policies and reporters; on its own `impl` block, its input adapters, ID generator and service providers.

On the struct, every argument is optional:

1. `name = "..."`: the workflow's name, which is otherwise the type's name as written;
2. `policies = [A, B]`: its workflow policies, in order;
3. `reporters = [A, B]`: reporters made with `Default` for every journey;
4. `async_reporters = [A, B]`: asynchronous reporters made the same way, in an asynchronous workflow only.

The workflow takes its mode from its steps: it is asynchronous when they are, their `run` an `async fn`, and synchronous otherwise. Its steps are all synchronous or all asynchronous.

Below `#[workflow(...)]`, one `#[with_step(...)]` line per step, in the order the steps run, gives the step's type, then `name = "..."`, which is required, and optionally `retry_budget = N`, `no_retry_on_abnormal_termination`, which keeps an abnormal termination from being retried, `policies = [A, B]` and `rescue = R`, which gives the step the rescue step `R`, run under the step's name followed by `_rescue`. One step type may be listed twice under two names.

The attribute implements `Workflow`, whose `descriptor()` is built the first time it is asked for and shared by every instance; with `Workflow` in scope, `Orders::descriptor().listing()` lists the workflow's steps. It also gives the workflow `builder()`, the builder of its instances. The builder has one setter per field of the struct, named after it, then `initial_data`, `reporters_global_metadata`, `reporter` (and `async_reporter` in an asynchronous workflow), `fake_provider`, which takes a fake in place of a service provider with itinera's `testing` feature, and `create`. A field is required unless its type is an `Option` or it is marked `#[default]`, in which case it starts from its type's default; calling `create` before every required field is set does not compile. The fields are the workflow's own state, never journey data. They have names, and none is named like a method of the builder: `initial_data`, `reporters_global_metadata`, `reporter`, `async_reporter`, `fake_provider` or `create`.

Mistakes in the declaration are compile errors, including every violation the descriptor's `build()` would report: two steps of one name, counting the names of rescue steps, a rescued step with a retry budget, two policies of one step, or two workflow policies, defining the same hook, an input adapter for a step the workflow does not list, counting the names of rescue steps, and a step adapted twice. So are a workflow without steps, a step without a name, two input adapters of one name, two ID generators, steps of both modes, and an asynchronous policy or reporter in a workflow whose steps are synchronous. A generic struct is refused too, since its descriptor could not be shared: declare such a workflow with `WorkflowDescriptor::builder`.

On the workflow's own `impl` block, `#[workflow]` takes no arguments and adds the methods marked as parts of the declaration; the block is needed only when there is one. Each takes the workflow as `&self` and is synchronous:

1. `#[input_adapter(steps = ["a", "b"])]` makes the method an input adapter of the listed steps, which may name a rescue step, such as `"charge_rescue"` for the rescue step of `charge`. It is named after the method unless `name = "..."` is given. Its parameters request what it reads, each marked with its attribute: `#[step_name]` with `StepName`, `#[key]` with `&str`, `#[from_workflow]` with a value's type or an `Option` of it, `#[journey_id]` with `&JourneyId`, and `#[data_bag]` with `DataBagAccess<'_>`. `#[from_workflow]` reads the key named after the parameter, or the one it names, as in `#[from_workflow("price")]`; an `Option` is a key that may be absent, unless `#[from_workflow(required)]` makes the key hold the `Option` itself. It returns `Result<Supplied, Error>`.
2. `#[id_generator]` makes the method the workflow's ID generator. It may take the initial data as `&DataBag`, and returns `Result<String, Error>`.

Every function of the block may use the shorthands for what an input adapter returns that `Workflow` gives: `Self::supply(value)` for the value it supplies, and `Self::from_data_bag()` to leave the input to the data bag.

# Examples

A workflow with a field the builder requires, one it may leave unset, an input adapter and an ID generator:

```
use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::instance::WorkflowInstance;
use itinera::journey::InitialData;
use itinera::step::{Outcome, Step, step};
use itinera::workflow::{Supplied, Workflow, workflow};

#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    fn run(self) -> Result<Outcome, Error> {
        if self.amount > 0 {
            Self::succeed()
        } else {
            Self::fail("nothing-to-charge")
        }
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", retry_budget = 2)]
struct Orders {
    shop: &'static str,
    discount: Option<i64>,
}

#[workflow]
impl Orders {
    #[input_adapter(steps = ["charge"])]
    fn pricing(&self, #[from_workflow] price: i64) -> Result<Supplied, Error> {
        let discount = self.discount.unwrap_or_default();
        Self::supply(price - discount)
    }

    #[id_generator]
    fn order_id(&self) -> Result<String, Error> {
        Ok(format!("{}-1", self.shop))
    }
}

#[derive(InitialData)]
struct NewOrder {
    price: i64,
}

let instance = Orders::builder()
    .shop("corner")
    .initial_data(NewOrder { price: 40 })
    .create()?;
assert_eq!(instance.journey_id().to_string(), "corner-1");
assert_eq!(Orders::descriptor().listing().len(), 1);
let result = LocalExecutor::new().run(instance)?;
assert!(result.status.is_succeeded());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Leaving a required field unset does not compile:

```compile_fail
use itinera::error::Error;
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

#[derive(Step)]
struct Ship;

#[step]
impl Ship {
    fn run(self) -> Result<Outcome, Error> {
        Self::succeed()
    }
}

#[workflow]
#[with_step(Ship, name = "ship")]
struct Orders {
    shop: &'static str,
}

let instance = Orders::builder().create();
```
