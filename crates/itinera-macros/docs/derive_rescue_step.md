Builds a rescue step from its fields, each time it runs: every field is one of its requests, resolved in the order the fields are written.

Each field is one of these, the first two marked with their attribute and the others recognised by their type:

- `#[input]` or `#[input("key")]`: an input, whose key is the field's name or the one given, as for a step. The input adapter attached to the rescue step's name, such as `steps = ["charge_rescue"]`, supplies it if it returns a value; otherwise it is read from the data bag. An adapter attached to the step it rescues is never asked.
- `#[from_rescued_step]` or `#[from_rescued_step("key")]`: data the failed attempt of the step it rescues contributed under the key. That data was never committed to the data bag.
- A `Reason`: the reason of the failure that ended the attempt.
- An `Error`: the error that ended the attempt abnormally, handed over by value.

The macro reads only the type as written, by the last segment of its path, so an alias of `Reason` or `Error` is not recognised, and an unmarked field of another type whose path ends in `Reason` or `Error`, such as `std::io::Error`, is taken as that request and fails to compile.

The field's type says whether a request is optional. A field of type `Option<T>` is an optional request: `None` when there is nothing to receive, such as no reason after an abnormal termination. A field of any other type is a required request, and when there is nothing to receive the journey aborts with `required data missing`, naming the rescue step. `#[input(required)]` and `#[from_rescued_step(required)]`, on a field of an `Option` type, request a key that must be present, though its value is itself an `Option`. The macro reads only the tokens written, so an alias of `Option` is a required request.

The struct has named fields, or none, and is not generic. The derive implements `FromRescueRequests`; the block that holds the rescue step's `run` carries `#[rescue_step]`.

# Examples

A step whose first attempt fails, rescued by a step that reads what that attempt contributed and the reason it gave, then attempted again:

```
use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::Contributor;
use itinera::step::{Outcome, Reason, RescueOutcome, RescueStep, Step, rescue_step, step};
use itinera::workflow::{Workflow, workflow};

#[derive(Step)]
struct Charge {
    #[input]
    token: Option<String>,
}

#[step]
impl Charge {
    fn run(self, contributor: &mut Contributor) -> Result<Outcome, Error> {
        if self.token.is_some() {
            return Self::succeed();
        }
        contributor.contribute("account", "A-7".to_owned());
        Self::fail("token-expired")
    }
}

#[derive(RescueStep)]
struct RefreshToken {
    #[from_rescued_step]
    account: String,
    reason: Option<Reason>,
    error: Option<Error>,
}

#[rescue_step]
impl RefreshToken {
    fn run(self, contributor: &mut Contributor) -> Result<RescueOutcome, Error> {
        if self.error.is_some() {
            return Self::fail("charge-crashed");
        }
        match self.reason {
            Some(reason) if reason.code() == "token-expired" => {}
            _ => return Self::fail("not-a-token-problem"),
        }
        contributor.contribute("token", format!("{}-fresh", self.account));
        Self::succeed()
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", rescue = RefreshToken)]
struct Orders;

let instance = Orders::builder().create()?;
let result = LocalExecutor::new().run(instance)?;
assert!(result.status.is_succeeded());
let listing = Orders::descriptor().listing();
let rescue = listing.first().and_then(|step| step.rescue.as_ref());
assert_eq!(rescue.map(|rescue| rescue.step.to_string()), Some("charge_rescue".to_owned()));
# Ok::<(), Box<dyn std::error::Error>>(())
```
