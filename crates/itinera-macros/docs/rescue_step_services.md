# Services

`run` may also take services, marked `#[service]` as a step's are: by shared reference, as a trait object such as `&dyn PaymentGateway`, for the rescue step's run only, with its key the parameter's name unless the attribute gives it. The rescue step's needs declare them after its handles, in the order of the parameters. Taking a key twice does not compile, nor does a service taken by value or by mutable reference, or as a concrete type.

A rescue step is served by the workflow's provider attached to its name, such as `steps = ["charge_rescue"]` for the rescue step of `charge`, or else by the one attached to no step, never by one attached to the step it rescues. Its services' keys are listed in its `DescribedRescueStep`, and their types in the bounds of a hidden function of the rescue step, so that a workflow the macros declare does not compile when no provider serves one of them, or when its providers make another type than the rescue step requests.

```
use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::step::{DescribedRescueStep, RescueOutcome, RescueStep, rescue_step};

trait TokenService: Send + Sync {
    fn token(&self, client: &str) -> String;
}

#[derive(RescueStep)]
struct RefreshToken {
    #[input]
    client: String,
}

#[rescue_step]
impl RefreshToken {
    fn run(
        self,
        #[service] tokens: &dyn TokenService,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<RescueOutcome, Error> {
        contributor.contribute("token", tokens.token(&self.client));
        Self::succeed()
    }
}

assert_eq!(RefreshToken::SERVICES, ["tokens"]);
```
