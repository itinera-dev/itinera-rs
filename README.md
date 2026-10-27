# itinera

Itinera is a workflow framework that keeps business rules separate from flow control. Each business rule lives in a step, and flow control, such as ending a journey early when a step fails, lives in policies. A workflow puts them together, with how many times each step may be retried, and an executor runs a workflow instance of it, one journey per instance.

`itinera` is the Rust implementation of the [Itinera specification](https://github.com/itinera-dev/spec), whose vocabulary it uses.

## Quick start

Add the crate:

```sh
cargo add itinera
```

Then a first workflow: one step, which charges an order's price, with a step policy that audits each charge. The order is given as the journey's initial data, and the receipt is read from its output.

```rust
use itinera::error::Result;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, InitialData, Output};
use itinera::policy::{OnSuccess, step_policy};
use itinera::step::{Outcome, Step, step};
use itinera::workflow::workflow;

#[derive(Step)]
struct Charge {
    #[input]
    price: i64,
}

#[step]
impl Charge {
    fn run(self, contributor: &mut Contributor) -> Result<Outcome> {
        if self.price <= 0 {
            return Self::fail("nothing-to-charge");
        }
        contributor.contribute("charged", self.price);
        Self::succeed()
    }
}

#[derive(Default)]
struct Audit;

#[step_policy(name = "audit")]
impl Audit {
    #[on_step_success]
    fn audited(
        &self,
        #[from_step] charged: i64,
        contributor: &mut Contributor,
    ) -> Result<OnSuccess> {
        contributor.contribute("audit", format!("charged {charged}"));
        Self::keep_default()
    }
}

#[workflow(name = "orders")]
#[with_step(Charge, name = "charge", retry_budget = 2, policies = [Audit])]
struct Orders;

#[derive(InitialData)]
struct NewOrder {
    price: i64,
}

#[derive(Output)]
struct Receipt {
    charged: i64,
    audit: String,
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let instance = Orders::builder()
        .initial_data(NewOrder { price: 40 })
        .create()?;
    let receipt: Receipt = LocalExecutor::new().run(instance)?.output()?;
    println!("{} ({})", receipt.charged, receipt.audit);
    Ok(())
}
```

It prints `40 (charged 40)`. The crate's [examples](https://github.com/itinera-dev/itinera-rs/tree/main/crates/itinera/examples) go further: `orders` adds a second step, a payment service and its provider, a workflow policy using a role, an input adapter, an ID generator and a reporter; `orders_async`, which needs the `async` feature, runs the same workflow on the asynchronous executor; and `orders_by_hand` declares it with the builder alone, without macros.

## The parts of a workflow

- A **step** holds one business rule. It declares the data it needs, which the workflow supplies, runs once per attempt, and reports an outcome: success, failure or skipped. Only an attempt that succeeds commits what the step contributed. A step may have a **rescue step**, which runs when the step fails or terminates abnormally and, once it succeeds, lets the step run again. See [`step`](https://docs.rs/itinera/latest/itinera/step/).
- A **workflow** lists its steps in the order they run, with their retry budgets and step policies, its workflow policies, its input adapters, its service providers, the roles it provides, its ID generator and its reporters. It is declared once, before any journey, and a declaration that breaks a rule of the specification is refused. See [`workflow`](https://docs.rs/itinera/latest/itinera/workflow/).
- A **step policy** holds flow control for the steps it is attached to. Its hooks, `on step success`, `on step failure`, `on step retry` and `on step abnormal termination`, are called in response to a step's outcome, and may end the journey early. See [`policy`](https://docs.rs/itinera/latest/itinera/policy/).
- A **workflow policy** acts when the journey ends: its hooks, `on workflow success` and `on workflow failure`, are called before the result is produced, and what they contribute is part of the journey's output. Any hook may use a role, an interface of operations the workflow provides, such as sending a notification. See [`policy`](https://docs.rs/itinera/latest/itinera/policy/).
- What happens in a journey is recorded as **events**, which the journey's reporters receive, together with the journey's **global metadata**: entries of a key and a value, apart from the data bag, starting with the journey ID. See [`event`](https://docs.rs/itinera/latest/itinera/event/) and [`report`](https://docs.rs/itinera/latest/itinera/report/).
- A **service** is an object of operations a step uses to do its business, such as a payment gateway. The step requests it by key, and a service provider of the workflow makes it, for the journey or for each attempt. See [`service`](https://docs.rs/itinera/latest/itinera/service/).
- A **workflow instance** is created for each journey, with its initial data, its reporters and any initial global metadata, and an **executor** runs it to a **result**: the journey succeeded, failed or was aborted. See [`instance`](https://docs.rs/itinera/latest/itinera/instance/), [`executor`](https://docs.rs/itinera/latest/itinera/executor/) and [`journey`](https://docs.rs/itinera/latest/itinera/journey/).

## Learning more

- [The Itinera book for Rust](https://itinera-dev.org/book/rust/) teaches Itinera from a first workflow onwards, and every example in it compiles and runs.
- [The API reference](https://docs.rs/itinera/latest/itinera/), on docs.rs, documents each module and item, with an example.
- [The Itinera site](https://itinera-dev.org/) explains why Itinera exists.

## Status

`0.1.0-beta.5` implements tier 1 of specification 0.1.0, with the capabilities `sync`, `async` and `testing`, and passes the conformance cases `v0.1.0-rc.16`. It is a beta, not yet meant for production: until `0.1.0`, features may be added and its API may still change. The [changelog](https://github.com/itinera-dev/itinera-rs/blob/main/CHANGELOG.md) says what each release implements, and each [GitHub release](https://github.com/itinera-dev/itinera-rs/releases) carries the report of the conformance cases it passes. The minimum supported Rust version is 1.85.

## Features

| Feature | Default | Adds |
|---|---|---|
| `macros` | on | The attributes and derives that declare steps, rescue steps, policies, workflows, roles and journey data. They expand to builder calls and implementations of public traits, which an application can write by hand |
| `async` | off | The asynchronous executor, and asynchronous steps, rescue steps, hooks, service providers, reporters and dispatchers, with `mode::async_trait` for services and roles whose methods are asynchronous |
| `testing` | off | Test support: `report::RecordingReporter`, which keeps every event of a journey, and the latest value of each entry of its global metadata, for a test to read; fakes in place of service providers; scripted runs; and recorders that lend a step, a hook or an input adapter its handles outside a journey. Enable it under `[dev-dependencies]` |
| `unstable` | off | API the specification has not settled yet, which may change or go; empty for now |

`itinera` brings in `itinera-core`, which holds the implementation, and, with `macros` or `async`, `itinera-macros`. Neither needs to be added directly.

## Rules made impossible to express

The specification lets an implementation make some rules impossible to express rather than check them while a workflow runs. In Rust, these mistakes are compile errors:

- a hook returning a lifecycle it may not return;
- a policy needing a role the workflow does not provide;
- an asynchronous part run by the synchronous executor, or an asynchronous service provider in a synchronous workflow;
- a rescue step that skips;
- a service provider of journey scope requesting the step being built or the attempt, or a service provider returning nothing instead of an instance or an error;
- something that is not a value entering the data bag, event data, a reason's details or global metadata;
- a contributor or reporter used after its attempt or hook has ended, or an input adapter's access to the data bag used after its call;
- an input adapter writing to the data bag through its access;
- a test's script choosing a negative attempt or a fraction of one.

## Contributing

The [contributor guide](https://github.com/itinera-dev/itinera-rs/blob/main/docs/contributing.md) says how work arrives in the repository, how itinera-rs is put together, how to build and test it and how to run the conformance cases. Itinera is built with AI under the terms of [A manifesto for software engineering with AI](https://marlon-sousa.com/blog/manifesto/); see [how Itinera is built](https://github.com/itinera-dev/.github/blob/main/CONTRIBUTING.md#how-itinera-is-built).

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/itinera-dev/itinera-rs/blob/main/LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](https://github.com/itinera-dev/itinera-rs/blob/main/LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
