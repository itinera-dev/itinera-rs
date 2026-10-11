Reads a journey's output as a struct, with `result.output::<T>()`: each field is read from one key of the data bag of a journey that succeeded or failed.

A field's key is its name, unless `#[key("...")]` names it. Its type is a value. A field of type `Option<T>` is `None` when the key is absent, unless it is marked `#[required]`: the key must then be present, and holds the `Option` itself. `#[required]` on a field of any other type does not compile, since its key is required already. The macro reads only the tokens written, so an alias of `Option` is read as a required key.

Reading fails with an `OutputError` naming the key when a required key is absent or a key holds a value of another type, and when the journey was aborted, since an aborted journey has no data bag.

The struct has named fields, or none, and is not generic. The derive implements `Output`, which an application may also implement by hand.

# Examples

```
use itinera::executor::LocalExecutor;
use itinera::journey::Output;
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

#[derive(Output)]
struct Receipt {
    price: i64,
    #[key("discount_code")]
    code: Option<String>,
}

struct Orders;

let orders = WorkflowDescriptor::builder("orders")
    .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    .build()?;
let instance = orders.instance(Orders).data("price", 42_i64).create()?;
let result = LocalExecutor::new().run(instance)?;
let receipt = result.output::<Receipt>()?;
assert_eq!(receipt.price, 42);
assert_eq!(receipt.code, None);
# Ok::<(), Box<dyn std::error::Error>>(())
```
