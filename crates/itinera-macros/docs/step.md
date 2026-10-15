Describes a step from the block that holds its `run`, so that a workflow can list the step by its type, under the name the workflow gives it.

`run` takes the step by value, then the handles it uses while it runs, each marked with what it is:

- `#[contributor]` on a `&mut Contributor`, to add data to the data bag;
- `#[reporter]` on a `StepReporter<'_>`, or on an `AsyncStepReporter<'_>` in an asynchronous step, to emit the step's own events.

It returns `Result<Outcome, Error>`. Every function of the block may use the shorthands for the step's outcomes that `DescribedStep` gives, such as `Self::succeed()`, `Self::fail("declined")`, `Self::skip()` and `Self::skip_because(...)`. A step belongs only to workflows of its own mode: an `async fn run` makes it asynchronous, which needs the `async` feature.

The step is built anew for each attempt, from its inputs. Either its struct derives `Step`, or this block holds a `new` whose parameters are the inputs, each marked `#[input]` in the forms the derive takes. `new` returns `Self`, or a `Result` whose error means the step could not be built. The macro reads only the tokens written, so a `Result` is recognised only when its name is written `Result`, not through an alias of another name. A step whose struct derives `Step` and whose block also holds a `new` does not compile, nor does a step with neither.

The attribute takes no arguments, and goes on the step's own `impl` block. A generic step is described with the builder instead. The attribute implements `DescribedStep` for the step, and `FromInputs` from its `new` when the block holds one.

# Examples

A step built from its fields:

```
use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::step::{DescribedStep, Outcome, Step, StepReporter, step, step_name};

#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    fn run(
        self,
        #[contributor] contributor: &mut Contributor,
        #[reporter] mut reporter: StepReporter<'_>,
    ) -> Result<Outcome, Error> {
        if self.amount <= 0 {
            return Self::fail("nothing-to-charge");
        }
        reporter.info("charging")?;
        contributor.contribute("receipt", format!("R-{}", self.amount));
        Self::succeed()
    }
}

struct Orders;

let charge = Charge::descriptor::<Orders>(step_name!("charge"));
assert_eq!(charge.name(), step_name!("charge"));
```

An asynchronous step with its own `new`, which refuses to build it from a note that is too long. `required` says that the key must be present, though its value is itself an `Option`:

```
use itinera::error::Error;
use itinera::step::{AsyncStepReporter, DescribedStep, Outcome, step, step_name};

struct Ship {
    gift_note: Option<String>,
}

#[step]
impl Ship {
    fn new(#[input(required)] gift_note: Option<String>) -> Result<Self, Error> {
        if gift_note.as_ref().is_some_and(String::is_empty) {
            return Err(Error::msg("a gift note is never empty"));
        }
        Ok(Self { gift_note })
    }

    async fn run(self, #[reporter] mut reporter: AsyncStepReporter<'_>) -> Result<Outcome, Error> {
        match self.gift_note {
            Some(note) => reporter.info_with("gift note", note).await?,
            None => return Self::skip_because("no-gift-note"),
        }
        Self::succeed()
    }
}

struct Orders;

let ship = Ship::descriptor::<Orders>(step_name!("ship"));
assert_eq!(ship.name(), step_name!("ship"));
```
