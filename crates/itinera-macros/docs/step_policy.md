Describes a step policy from the block that holds its hooks, so that a step can attach the policy by its type, with `Audit::descriptor()`.

Each hook is a method marked with its name, taking `&self`, then the parameters it requests:

| Marker | Returns |
| --- | --- |
| `#[on_step_success]` | `Result<OnSuccess, Error>` |
| `#[on_step_failure]` | `Result<OnFailure, Error>` |
| `#[on_step_retry]` | `Result<OnFailure, Error>` |
| `#[on_step_abnormal_termination]` | `Result<OnFailure, Error>` |

The method's own name is free. A block defines at least one hook, and each at most once. An `async fn` hook makes the policy asynchronous: it needs the `async` feature, every hook of the block is then asynchronous, and the policy is attached only to an asynchronous workflow's steps. Every function of the block may use the shorthands for what a hook returns, which `DescribedPolicy` gives: `Self::keep_default()`, `Self::fail_workflow("fraud")`, and, on step success only, `Self::finish_workflow()`.

Each parameter after `&self` requests one thing. Data and roles carry an attribute saying what they request:

| Attribute | Parameter type | Hooks |
| --- | --- | --- |
| `#[from_step]` or `#[from_step("key")]` | `T` or `Option<T>`, `T` a value | all |
| `#[from_workflow]` or `#[from_workflow("key")]` | `T` or `Option<T>`, `T` a value | all |
| `#[role]` | `&dyn R`, a role of the workflow | all |

Every other parameter is a handle, recognised by its type alone:

| Handle | Parameter type | Hooks |
| --- | --- | --- |
| the step's name | `StepName` | all |
| the attempt | `Attempt` | all |
| the reason | `Reason` or `Option<Reason>` | on step failure, on step retry |
| the cause | `StepFailureCause` on step failure, `RetryCause` on step retry | on step failure, on step retry |
| the error | `&Error` or `Option<&Error>` | on step failure, on step retry, on step abnormal termination |
| the journey ID | `&JourneyId` | all |
| a contributor | `&mut Contributor` | all |
| a reporter | `HookReporter<'_>`, or `AsyncHookReporter<'_>` in an asynchronous hook | all |

The macro reads only the type as written, by the last segment of its path, so an alias of one of these types is not recognised.

A key is the parameter's name unless the attribute names it. As with a step's inputs, an `Option` type makes a request optional, and `required`, as in `#[from_step("note", required)]`, makes the key required though its value is itself an `Option`. Requests resolve in the order of the parameters. A role requested makes the policy one that only a workflow providing that role, with `Provides`, can attach. An asynchronous hook holds its parameters across its awaits, so a role it requests is `Sync`, as a role trait declared `trait Notifier: Sync` is.

The policy is named after its type as written, unless the attribute names it: `#[step_policy(name = "audit")]`. It is built for each journey with `Default`, unless one function of the block, marked `#[factory]`, takes no parameters and returns `Self`, or a `Result` whose error means the policy could not be built. A generic policy is described with the builder instead.

The attribute implements `DescribedPolicy`, `StepPolicy` for every workflow providing the roles requested, and the trait of each hook it defines.

# Examples

```
use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::policy::{OnFailure, OnSuccess, Provides, StepPolicy, step_policy};
use itinera::step::{Outcome, Reason, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

trait Notifier {
    fn notify(&self, message: &str);
}

struct Orders;

impl Notifier for Orders {
    fn notify(&self, _message: &str) {}
}

impl Provides<dyn Notifier> for Orders {
    fn role(&self) -> &(dyn Notifier + 'static) {
        self
    }
}

#[derive(Default)]
struct Audit;

#[step_policy(name = "audit")]
impl Audit {
    #[on_step_success]
    fn checked(&self, #[from_step] receipt: Option<String>) -> Result<OnSuccess, Error> {
        match receipt {
            Some(_) => Self::keep_default(),
            None => Self::fail_workflow("no-receipt"),
        }
    }

    #[on_step_failure]
    fn reported(
        &self,
        reason: Reason,
        #[role] notifier: &dyn Notifier,
    ) -> Result<OnFailure, Error> {
        notifier.notify(reason.code());
        Self::keep_default()
    }
}

let charge = StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success()))
    .policy(Audit::descriptor());
let orders = WorkflowDescriptor::builder("orders").step(charge).build()?;
let result = LocalExecutor::new().run(orders.instance(Orders).create()?)?;
assert!(result.status.is_failed());
# Ok::<(), Box<dyn std::error::Error>>(())
```
