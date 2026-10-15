Builds a step from its fields, for each attempt: every field is one of its inputs, or a value made with `Default`.

Each field carries one of these attributes:

- `#[input]`: the input whose key is the field's name;
- `#[input("key")]`: the input with that key;
- `#[input(required)]` or `#[input("key", required)]`: on a field of an `Option` type, the input whose key must be present, though its value is itself an `Option`;
- `#[default]`: not an input; the field starts with `Default::default()` at every attempt.

The field's type says whether an input is optional. A field of type `Option<T>` is an optional input: `None` when the key is absent. A field of any other type is a required input, and `required` on it does not compile. The macro reads only the tokens written, so an alias of `Option` is a required input.

The struct has named fields, or none, and is not generic. The standard `#[derive(Default)]` also reads `#[default]`, so a struct deriving `Step` with a field marked `#[default]` cannot derive `Default` too. The derive implements `FromInputs`; the block that holds the step's `run` carries `#[step]`.

# Examples

```
use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, JourneyStatus};
use itinera::step::{DescribedStep, Outcome, Step, step, step_name};
use itinera::workflow::WorkflowDescriptor;

#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
    #[input("discount_code")]
    code: Option<String>,
    #[default]
    fees: Vec<i64>,
}

#[step]
impl Charge {
    fn run(mut self, #[contributor] mut contributor: Contributor<'_>) -> Result<Outcome, Error> {
        if self.code.is_none() {
            self.fees.push(2);
        }
        let total = self.amount + self.fees.iter().sum::<i64>();
        contributor.contribute("total", total);
        Self::succeed()
    }
}

struct Orders;

let orders = WorkflowDescriptor::builder("orders")
    .step(Charge::descriptor(step_name!("charge")))
    .build()?;
let instance = orders.instance(Orders).data("amount", 40_i64).create()?;
let result = LocalExecutor::new().run(instance)?;
assert!(result.status.is_succeeded());
# Ok::<(), Box<dyn std::error::Error>>(())
```
