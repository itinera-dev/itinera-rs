
# Services

With the `unstable` feature of `itinera`, `run` and `new` may also take services, which the workflow's service providers supply. A parameter marked `#[service]` takes the service by shared reference, as the type it is lent as, such as `&dyn PaymentGateway`, for the attempt only. Its key is the parameter's name, unless the attribute gives it, as in `#[service("payments")]`. The step's needs declare the services `new` takes among its inputs, and those only `run` takes after its handles, each in the order of the parameters; the services are resolved, and listed, in that order. A service that `new` and `run` both take is declared once, and is the same type in both; one function taking a key twice does not compile, nor does a service taken by value or by mutable reference.

The services' keys are listed in the step's `DescribedStep`, and their types in the bounds of a hidden function of the step, so that a workflow the macros declare does not compile when no provider serves one of them, or when its providers make another type than the step requests.

```
use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::step::{DescribedStep, Outcome, step, step_name};

trait PaymentGateway: Send + Sync {
    fn charge(&self, amount: i64) -> String;
}

trait StockStore: Send + Sync {
    fn reserve(&self, amount: i64) -> bool;
}

struct Charge {
    amount: i64,
}

#[step]
impl Charge {
    fn new(#[input] amount: i64, #[service] stock: &dyn StockStore) -> Result<Self, Error> {
        if stock.reserve(amount) {
            Ok(Self { amount })
        } else {
            Err(Error::msg("out of stock"))
        }
    }

    fn run(
        self,
        #[service("payments")] gateway: &dyn PaymentGateway,
        #[contributor] mut contributor: Contributor<'_>,
    ) -> Result<Outcome, Error> {
        contributor.contribute("receipt", gateway.charge(self.amount));
        Self::succeed()
    }
}

struct Orders;

assert_eq!(Charge::SERVICES, ["stock", "payments"]);
let charge = Charge::descriptor::<Orders>(step_name!("charge"));
assert_eq!(charge.name(), step_name!("charge"));
```
