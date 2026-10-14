Describes a rescue step from the block that holds its `run`, so that a workflow can give it to a step by its type. A rescue step has no name of its own: it runs under the name of the step it rescues followed by `_rescue`.

`run` takes the rescue step by value, then the handles it uses while it runs, each marked with what it is, as a step's `run` does:

- `#[contributor]` on a `Contributor<'_>`, to add data to the data bag;
- `#[reporter]` on a `StepReporter<'_>`, or on an `AsyncStepReporter<'_>` in an asynchronous rescue step, to emit its own events.

It returns `Result<RescueOutcome, Error>`. Every function of the block may use the shorthands for a rescue step's outcomes that `DescribedRescueStep` gives: `Self::succeed()` and `Self::fail("declined")`. A rescue step never skips, so there is no shorthand for it. A rescue step belongs only to steps of workflows of its own mode: an `async fn run` makes it asynchronous, which needs the `async` feature.

The rescue step is built from its requests each time it runs, and its struct derives `RescueStep`, which says what it requests. The attribute takes no arguments, and goes on the rescue step's own `impl` block. A generic rescue step is given with the builder instead. The attribute implements `DescribedRescueStep` for the rescue step.

# Examples

```
use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::step::{
    Outcome, RescueOutcome, RescueStep, StepDescriptor, StepReporter, rescue_step, step_name,
};
use itinera::workflow::WorkflowDescriptor;

#[derive(RescueStep)]
struct RefreshToken {
    #[input]
    client: String,
}

#[rescue_step]
impl RefreshToken {
    fn run(
        self,
        #[contributor] mut contributor: Contributor<'_>,
        #[reporter] mut reporter: StepReporter<'_>,
    ) -> Result<RescueOutcome, Error> {
        if self.client.is_empty() {
            return Self::fail("no-client");
        }
        reporter.info("refreshing the token")?;
        contributor.contribute("token", format!("{}-fresh", self.client));
        Self::succeed()
    }
}

struct Orders;

let charge = StepDescriptor::<Orders>::new(step_name!("charge"), || Ok(Outcome::success()))
    .rescued_by::<RefreshToken>();
let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
let listing = orders.listing();
let rescue = listing.first().and_then(|step| step.rescue.as_ref());
assert_eq!(rescue.map(|rescue| rescue.step), Some(step_name!("charge_rescue")));
# Ok::<(), Box<dyn std::error::Error>>(())
```

A rescue step that skips does not compile:

```compile_fail
use itinera::error::Error;
use itinera::step::{RescueOutcome, RescueStep, rescue_step};

#[derive(RescueStep)]
struct RefreshToken;

#[rescue_step]
impl RefreshToken {
    fn run(self) -> Result<RescueOutcome, Error> {
        Self::skip()
    }
}
```
