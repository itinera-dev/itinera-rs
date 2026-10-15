Describes a workflow policy from the block that holds its hooks, so that a workflow can attach the policy by its type, with `Notify::descriptor()`.

Each hook is a method marked with its name, taking `&self`, then the parameters it requests, and returning `Result<(), Error>`: `#[on_workflow_success]` and `#[on_workflow_failure]`. A workflow hook makes no lifecycle decision, and an `Err` aborts the journey with `hook failed`. Its parameters request what `#[step_policy]` describes that every hook may request: `#[from_workflow]`, `#[role]`, and the handles `&JourneyId`, `&mut Contributor` and a reporter.

Everything else is as `#[step_policy]` says: the hooks defined, an asynchronous policy, the name, `#[factory]`, and generic policies. The attribute implements `DescribedPolicy`, `WorkflowPolicy` for every workflow providing the roles requested, and the trait of each hook it defines.

# Examples

A workflow policy whose factory may fail:

```
use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::JourneyId;
use itinera::policy::{HookReporter, WorkflowPolicy, workflow_policy};
use itinera::step::{Outcome, StepDescriptor, step_name};
use itinera::workflow::WorkflowDescriptor;

struct Notify {
    channel: &'static str,
}

#[workflow_policy(name = "notify")]
impl Notify {
    #[factory]
    fn connected() -> Result<Self, Error> {
        Ok(Self { channel: "orders" })
    }

    #[on_workflow_success]
    fn announce(
        &self,
        journey_id: &JourneyId,
        #[from_workflow("total")] total: i64,
        mut reporter: HookReporter<'_>,
    ) -> Result<(), Error> {
        reporter.info(format!("{journey_id} paid {total} on {}", self.channel))?;
        Ok(())
    }
}

struct Orders;

let orders = WorkflowDescriptor::builder("orders")
    .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    .policy(Notify::descriptor())
    .build()?;
let instance = orders.instance(Orders).data("total", 40_i64).create()?;
let result = LocalExecutor::new().run(instance)?;
assert!(result.status.is_succeeded());
# Ok::<(), Box<dyn std::error::Error>>(())
```
