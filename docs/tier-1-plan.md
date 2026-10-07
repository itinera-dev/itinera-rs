# Tier 1 plan

How itinera-rs implements tier 1 of specification 0.1.0: the architecture, the tests, the order of work, the issues, the tooling and the release. Tracked in [#7](https://github.com/itinera-dev/itinera-rs/issues/7).

This document holds what crosses proposals. What concerns a single proposal goes in its tech spec, `docs/specs/NNNN-short-name.md`, which refers here for the rest. Behaviour is defined only by the [specification](https://github.com/itinera-dev/spec/tree/main/spec): where this plan and the specification disagree, the specification is right, and the plan is corrected.

Code samples show the intended shape. Names of attributes and methods may still change while implementing; the tech specs record the final ones.

## What this plan implements

- **Specification 0.1.0, tier 1**: proposals 0002, 0008, 0009, 0010, 0011, 0012, 0024, 0027, 0032, 0040, 0041, 0042, 0049, 0054, 0055, 0056, 0057, 0058, 0060, 0061, 0062, 0063, 0064, 0065, 0081, 0083 and 0085.
- **The cases** at [itinera-dev/conformance](https://github.com/itinera-dev/conformance) `v0.1.0-rc.5`, and later candidates as they are tagged.
- **Capabilities claimed**: `sync` and `async`.
- **Rules made impossible to express** (proposal 0054): `invalid-lifecycle`, `role-not-provided`, `mode-not-accepted`, `non-value` and `late-handle`.

How excluded scenarios are declared and reported is defined in the conformance repository's FORMAT.md: the manifest lists each one with its proof, and the `run-conformance` action reports them next to the Cucumber JSON (decision 11).

## Architecture

### 1. Crates

A Cargo workspace, edition 2024, resolver 3, under `crates/`.

| Crate | Published | Holds |
|---|---|---|
| `itinera` | yes | What applications depend on: re-exports `itinera-core`, and the macros behind the `macros` feature |
| `itinera-core` | yes | Values and the data bag, events, reporters and dispatchers, steps, policies, descriptors and their builder, the workflow instance, the engine and both local executors |
| `itinera-macros` | yes | The procedural macros |
| `itinera-conformance` | no | The conformance runner |

There is no separate executor crate in tier 1: both executors share one engine, which stays private. Executors of later tiers, such as durable ones, get crates of their own.

### 2. A typed builder, and rules made impossible to express

The builder is the public API, and the macros are only syntax over it. There is no untyped API. Five rules are made impossible to express, each proven by a test (decision 11):

- **Lifecycles.** Each hook has its own return type, holding only the lifecycles it may return, so an invalid lifecycle cannot be written.
- **Roles.** A workflow is generic over its own type `W`, and a policy that needs a role is implemented only for workflows whose `W` implements that role's trait. Attaching it to a workflow without the role does not compile.
- **Execution modes.** The mode is part of the descriptor's type. Adding an asynchronous part turns a `Sync` descriptor into an `Async` one, and the synchronous executor accepts only `Sync`.
- **Values.** Only values can enter the data bag, event data or a reason's details (decision 3).
- **Late handles.** A contributor or reporter cannot outlive its attempt or hook (decision 4).

The rules types cannot reach are checked when the descriptor is built (decision 6) or while the journey runs.

### 3. Values and the data bag

- **A value** is any `T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static`. Closures, function pointers and handles cannot be values. Nothing is serialized by the engine: the bounds only make sure that something could serialize them.
- **The data bag** maps `String` keys to type-erased values, together with what is needed to clone and serialize them. Event data and reason details use the same type-erased value.
- **Reading** names the exact type `T`. Any other type is `wrong type`, including `i32` against `i64`.
- **Received data is read-only** because every receiver, step or hook, gets its own owned clone, never a reference into the bag. A type whose `Clone` shares mutable state, such as `Arc<Mutex<_>>`, defeats this, and the documentation says so.
- **Contributions are captured when made**: contributing takes the value by ownership.
- **The conformance runner** maps the neutral types to Rust types: `string` to `String`, `integer` to `i64`, `number` to `f64`, `boolean` to `bool`, `list` to `Vec<serde_json::Value>`, `object` to `serde_json::Map<String, Value>`.

### 4. Steps

- **A step's constructor declares everything it needs**, as annotated parameters, and asks only for what it uses: inputs, optional inputs, a `Contributor<'a>` and a `StepReporter<'a>`. The step keeps them as fields.

  ```rust
  #[step]
  impl<'a> Charge<'a> {
      fn new(
          #[input("amount")] amount: i64,
          #[input("discount", optional)] discount: Option<i64>,
          #[contributor] contributor: Contributor<'a>,
          #[reporter] reporter: StepReporter<'a>,
      ) -> Result<Self, Error> {
          Ok(Self { amount, discount, contributor, reporter })
      }
  }

  impl<'a> Step<'a> for Charge<'a> {
      fn run(self) -> Result<Outcome, Error> {
          self.reporter.info("charging")?;
          self.contributor.contribute("receipt", "R-1".to_string());
          Ok(Outcome::success())
      }
  }
  ```

- **`run(self)` takes nothing else.** It consumes the step, so nothing survives into another attempt. `Step` and `AsyncStep` are separate traits, so the mode is in the type.
- **Handles borrow their attempt.** `Contributor<'a>` and `StepReporter<'a>` carry the lifetime of the attempt, chosen by the executor, so they cannot be moved into a thread or task that outlives it. The guarantee comes from the types, with or without the macros.
- **Emitting can be interrupted.** `StepReporter` methods return `Result<(), Interrupted>`. `Interrupted` is the executor's signal that a reporter failed while the event was being delivered (decision 9); the step propagates it with `?`. `Contributor::contribute` returns nothing: it cannot fail, since only values can be contributed.
- **The builder takes a `StepFactory`**, which the macro implements:

  ```rust
  pub trait StepFactory: Send + Sync + 'static {
      type Step<'a>: Step<'a>;
      fn needs(&self) -> StepNeeds;
      fn build<'a>(&self, got: &mut Resolved<'a>) -> Result<Self::Step<'a>, Error>;
  }
  ```

  `StepNeeds` hands out typed tokens (`needs.input::<i64>("amount")`, `needs.contributor()`), and `got.take(token)` returns the resolved value or handle, already of the right type. A plain closure is accepted only for steps that use no handles.
- **Outcomes**: `Outcome::success()`, `Outcome::failure(reason)`, `Outcome::retriable_failure(reason)`, `Outcome::skipped()` and `Outcome::skipped_because(reason)`. A `Reason` has a code, an optional message and optional details, which are a value.
- **One error type, `itinera::error::Error`**, for every custom code that can fail: a step's `run` and constructor, hooks, role operations, policy factories, reporters and their `init`, dispatchers and dispatcher factories. It converts from any `std::error::Error + Send + Sync + 'static`, so `?` works on any library's error, and from `Interrupted`. Because of that conversion it does not itself implement `std::error::Error`; it offers `Display`, `source()` and `Error::msg`.
- **`?` in `run` is an abnormal termination**: an error the step did not anticipate. A failure the step chose is returned as `Outcome::failure`. The one exception is `Interrupted`: the engine recognises its own signal, the journey is already aborted, and nothing the step returns afterwards counts.
- **Contributions are kept per attempt**: visible to that attempt's hooks whatever its outcome, committed only on Success, dropped on an abnormal termination.

### 5. Policies, hooks and roles

- **Two kinds of policy**, `StepPolicy<W>` and `WorkflowPolicy<W>`. A policy type can hold only hooks of its own kind. Each policy has a name, which defaults to its type's name with the macros and is required with the builder.
- **Hooks declare what they need like steps do**, as annotated parameters. What a hook may request depends on its kind, so a request it may not make does not compile:

  | Request | Available to |
  |---|---|
  | data from the step, the step's name, the attempt number | step hooks |
  | the failure reason, as required (`Reason`) or optional (`Option<Reason>`) | `on step failure`, `on step retry` |
  | the cause, as its own enum per hook | `on step failure`, `on step retry` |
  | the error, as required (`&Error`) or optional (`Option<&Error>`) | `on step failure`, `on step retry`, `on step abnormal termination` |
  | data from the workflow, the journey ID, roles, a contributor, a reporter | every hook |

  A reason exists when the attempt reported a failure, and an error when it ended in an abnormal termination, whatever the hook and the cause. A required request for one that does not exist aborts with `required data missing`; an optional one gets `None`.

- **Each hook returns its own type, inside a `Result`**: `Result<Option<OnSuccess>, Error>` for `on step success` (`FinishWorkflow` or `FailWorkflow`), `Result<Option<FailWorkflow>, Error>` for `on step failure`, `on step retry` and `on step abnormal termination`, and `Result<(), Error>` for workflow hooks. `Err` aborts the journey with `hook failed`.
- **Hooks take `&self`.** Everything they need arrives as parameters, so they have no reason to change their policy.
- **Policies are built by the executor from factories** that return the policy or an `Error`: workflow policies once per journey, before it starts, where an error is a refusal; step policies for every attempt, with the step, where an error aborts the journey with `policy could not be built`. The builder also accepts factories that cannot fail.
- **Roles are plain traits**, implemented by the workflow's own type `W`. A hook requests one as `#[role] notifier: &dyn Notifier`. There is no marker trait. A role operation that can fail returns `Result<_, itinera::error::Error>`, and the hook propagates it, which aborts with `hook failed`.
- **Panics are never caught.** A panic is a bug: it propagates to whoever called `run`, and the journey stops as if the process had crashed.
- **Each hook has a synchronous trait and an asynchronous one**, behind the `async` feature. Any asynchronous part makes the workflow `Async`.

### 6. Workflow descriptors and input adapters

```rust
static ORDERS: LazyLock<WorkflowDescriptor<Orders, Sync>> = LazyLock::new(|| {
    WorkflowDescriptor::builder("orders")
        .step(StepDescriptor::new("charge", ChargeFactory)
            .retries(2)
            .abnormal_termination_retriable()
            .policy(|| Audit::default()))
        .step(StepDescriptor::new("ship", ShipFactory))
        .policy(|| Notify::new())
        .reporter::<AuditLog>()
        .id_generator(|w: &Orders, data: &InitialData| format!("order-{}", w.next_number()))
        .build()
        .expect("the orders workflow is well formed")
});
```

- **A workflow descriptor** is the workflow's declaration: its name, its step descriptors, its policy descriptors, its input adapters, its reporters and its ID generator. It is immutable, `Send + Sync`, cheap to clone, shared by every instance, and only `build()` can produce one.
- **Each step descriptor states everything**: the retry budget (0 unless set), `abnormal termination retriable` (false unless set), and its policies in the order attached.
- **`build()` checks what types cannot**, and returns every violation at once in `Violations`: `duplicate step name`, `hook defined twice`, `input adapter for unknown step` and `step adapted twice`.
- **`listing()`** gives each step's name, its position from 1, its policy names in order, and its input adapter's name.
- **Input adapters are hooks of the workflow**, written as its own annotated methods and attached to whole steps, at most one per step:

  ```rust
  #[workflow(name = "orders")]
  impl Orders {
      #[input_adapter(steps = ["charge", "refund"])]
      fn pricing(&self, #[step_name] step: &str, #[key] key: &str,
                 #[data_from_workflow("price")] price: i64) -> Result<Option<Value>, Error> { /* ... */ }
  }
  ```

  It is called for each input of an adapted step. A value is used; `None` means "not mine", and the input is read from the data bag; `Err` emits `input_adapter_failed` and aborts with `step could not be built`. Its value is untyped and checked against the step's declared type. It may request the step's name, the key, data from the workflow and the journey ID, never roles, a contributor or a reporter. The macro generates the builder call that registers it.

### 7. The workflow instance

The specification requires only that any executor can run any instance, however it was produced. In Rust, that contract is one trait, the only thing an executor relies on. The executor coordinates the flow; it does not own data.

```rust
pub trait WorkflowInstance: Send + 'static {
    type Workflow: Send + Sync + 'static;
    type Mode: Mode;
    fn descriptor(&self) -> &WorkflowDescriptor<Self::Workflow, Self::Mode>;
    fn workflow(&self) -> &Self::Workflow;
    fn journey_id(&self) -> &JourneyId;
    fn take_reporters(&mut self) -> Vec<BoxedReporter>;
    fn data_bag(&self) -> &DataBag;
    fn data_for_step(&self, step: &str, input: &InputRequest) -> Resolution;
    fn data_for_workflow(&self, request: &DataRequest) -> Resolution;
    fn commit(&mut self, key: String, value: AnyValue, source: Source) -> Committed;
}
```

- **The workflow's own type `W`** holds what its code needs, such as services, and implements its roles. Roles and input adapters see it; no hook ever sees the data bag.
- **itinera provides `Instance<W>`**, created through a builder that the macros also use:

  ```rust
  let instance = ORDERS.instance(Orders::new(services))
      .data("amount", 42_i64)
      .create()?;
  ```

  `create()` produces the journey ID first, with the descriptor's generator (a closure returning a `String`) or the default UUID version 4, then builds each reporter, then the instance. A failure there is an `InstanceError`, outside any journey.
- **Reporters are listed on the workflow by type.** Each implements `WorkflowReporter<W>`, whose `init(&W, &JourneyId, &InitialData) -> Result<Self, Error>` builds a new reporter for every instance.
- **The three data operations** are provided by itinera for `Instance<W>`. `data_for_step` applies the step's input adapter, then the data bag; `data_for_workflow` reads only the data bag; `commit` writes and says whether a value was replaced. Each answer carries an ordered report of what happened, which the engine turns into events and aborts.
- **A hand-written instance** implements the trait itself, points at a descriptor that only `build()` can produce, and may reuse itinera's data operations.
- **An instance runs once**: `run` takes it by value.

### 8. Executors

- **`LocalExecutor`** runs `Sync` workflows. **`AsyncLocalExecutor`**, behind the `async` feature, runs `Sync` and `Async` workflows, running synchronous parts inline.
- **One engine.** The scan, the decisions, the hooks and the events are written once, as asynchronous code. The synchronous executor drives it with `std::task::Waker::noop()`: a `Sync` workflow has nothing to wait for, so it runs straight through.
- **No runtime dependency.** The engine never spawns, sleeps, sets timers or cancels. It only awaits the futures of the workflow's own parts, one after another, so it runs on any async runtime.
- **`run` returns the journey's result, or a refusal before the journey starts**:

  ```rust
  impl<F: DispatcherFactory> LocalExecutor<F> {
      pub fn new() -> LocalExecutor<DefaultDispatcherFactory>;
      pub fn with_dispatcher_factory(factory: F) -> Self;
      pub fn run<I: WorkflowInstance<Mode = Sync>>(&mut self, instance: I) -> Result<JourneyResult, Refusal>;
  }
  ```

  A refusal comes from custom code called before the journey starts: a workflow policy that cannot be built, a dispatcher factory that fails, or a dispatcher that fails while the journey's reporters are added. Declaration violations are settled by `build()`, modes by types, and the journey ID and reporters by `create()`. `&mut self` makes concurrent journeys on one executor impossible to write.
- **Everything asynchronous is awaited**: asynchronous steps, hooks, reporters and dispatchers alike.
- **A dropped future abandons the journey.** If the caller drops the future returned by `AsyncLocalExecutor::run`, the journey stops where it is. The documentation says so.
- **Deferred:** cooperative yields between units of the async engine.

### 9. Events, reporters and dispatchers

- **One `Event` type**: a sequence number from 1, a `Timestamp` (a `SystemTime` that displays in ISO 8601 in UTC), the journey ID, the workflow name, and a typed body with one variant per event of the catalogue. `kind()` returns the event's snake_case name. No variant has a field able to hold a value from the data bag. Hook names are `StepHook` or `WorkflowHook`, and an event from or about a hook carries the step and attempt only for a step hook. Who decided follows from each decision, and names only a hook that may have decided it: `journey_failed` by `FailWorkflow` holds a `DecidingHook` (a policy and any step hook), `step_given_up` by `FailWorkflow` one whose hook is a `GiveUpHook` (`on step retry` or `on step abnormal termination`), and `journey_succeeded` by `FinishWorkflow` the policy whose `on step success` returned it. Every other decision, and every retry, is decided by default. Events are the domain's and reporters are its adapters, so events carry no format: neither `Event` nor any type it carries implements `Serialize`. Each reporter writes events as it chooses, from their fields, `kind()`, and names that display as the specification writes them. Only itinera constructs events: `Event` and its variants are `#[non_exhaustive]`, so code outside the crate can read them but not make them. The types events carry live with their concepts: `JourneyId`, `AbortReason`, `FailureCause` and `LastFailure` in `journey`, `StepAttempt` and `Reason` in `step`, the hook names and `Lifecycle` in `policy`. What only events need stays in `event`: who did or decided something, the decisions' causes, and `JourneyFailure` and `JourneyAbort`, which tell how a journey ended with errors as messages.
- **`journey_failed` and `journey_aborted` are events about the journey.** They name the step but carry no attempt number. `journey_failed` carries the cause, the reason when there is one and the error's message when there is one; `journey_aborted` has one variant per abort reason, holding the step when there is one, the details, and the error's message when failing custom code caused the abort. Events never carry an `itinera::error::Error`, only its `Display` text.
- **Data in step and hook events, and reason details,** are values carried in memory, cloned when emitted, and serialized only by reporters, in their own format.
- **Reporters**: `Reporter` with `report(&mut self, &Event) -> Result<(), Error>`, and `AsyncReporter` behind the `async` feature. A reporter is expected to handle its own trouble (log it, drop the event, retry later) and return `Ok`, and the documentation says so. An `Err` aborts the journey with `reporter failed`, and delivery of that event stops at that reporter: the reporters after it never receive it. `journey_aborted` then goes to every reporter except the one that failed, so the order of reporters matters, and the documentation says that too. The executor wraps each reporter it adds for the journey, so this holds whatever the dispatcher: once a reporter has failed, the wrappers let only `journey_aborted` through, and never to the one that failed. An error while `journey_aborted` itself is delivered is ignored. An asynchronous reporter makes the workflow `Async`.
- **The handles given to steps and hooks** are restricted views of the journey's dispatcher. A `StepReporter` emits only `step_info`, `step_warning` and `step_error`, stamped with the step and attempt; a `HookReporter` emits only `journey_info`, `journey_warning` and `journey_error`, stamped with the policy and hook. Delivery happens before the emit call returns. If a reporter fails during that delivery, the engine records the abort at once, and the call returns `Interrupted`. From then on nothing the step or hook emits is delivered, and when it returns, its outcome, lifecycle and contributions are ignored, even if it ignored `Interrupted` and carried on. In a workflow with an asynchronous reporter, the handles are asynchronous, and only asynchronous steps and hooks can request them.
- **Dispatchers**: `Dispatcher` and `AsyncDispatcher`, with `add` and `dispatch`, both returning `Result<(), Error>`. An executor is given a `DispatcherFactory`, or uses `DefaultDispatcherFactory`. The executor calls the factory once per journey, before the journey starts, adds the instance's reporters, and drops the dispatcher when the journey ends. `DefaultDispatcher` calls reporters in the order they were added, and stops at the first that fails, except on `journey_aborted`, which it delivers to every reporter.
- **One emitter** in the engine assigns sequence numbers and timestamps. Its clock can be replaced inside the crate for tests.

### 10. The result and the errors

```rust
pub struct JourneyResult {
    pub journey_id: JourneyId,
    pub status: JourneyStatus,
}

pub enum JourneyStatus {
    Succeeded { data: DataBag },
    Failed { failure: Failure, data: DataBag },
    Aborted(Abort),
}

pub enum Failure {
    Failure(Reason),
    RetriesExhausted(LastFailure<Error>),
    AbnormalTermination(Error),
    FailWorkflow(Reason),
}

pub enum Abort {
    StepCouldNotBeBuilt(Error),
    PolicyCouldNotBeBuilt { policy: String, error: Error },
    RequiredDataMissing(MissingData),
    WrongType { key: String, requester: Requester },
    HookFailed(Error),
    ReporterFailed(Error),
}
```

- **The result is a business outcome.** It holds no step statuses, attempt counts or step names: those are observable in the event stream.
- **The status is one enum carrying its payload**, so a failed journey without a failure, a succeeded one with an abort, or an aborted one with data, cannot be written. `JourneyStatus::kind()` returns a plain `StatusKind` (`Succeeded`, `Failed`, `Aborted`) for code that only needs the status.
- **An aborted journey has no data bag.** An abort means something illegal happened: it has no business outcome, and an aborted journey is never resumed. The data bag lives only in `Succeeded` and `Failed`, so reading the data of an aborted journey cannot be written. `JourneyStatus::data()` returns `Option<&DataBag>` for code that handles every status alike.
- **A failure has one variant per cause, and an abort one per abort reason**, each holding exactly what proposal 0083 says it carries: a reason, an error, or both, and the abort's details. `Failure::cause()` and `Abort::reason()` give the plain `FailureCause` and `AbortReason`. They mirror the events' `JourneyFailure`, `JourneyAbort`, `MissingData` and `Requester`, which hold the error's message and name the step, and share `LastFailure` with them, holding the error itself where events hold its message. The result's `MissingData` and `Requester` are the same shapes without the step's name, in `journey` with the result. `MissingData` is a key and who requested it, or a step hook's request for the failure's reason or the error, which have no key.
- **The result holds the error itself**, an `itinera::error::Error`, so the caller can inspect it; events carry only its message. As an error cannot be cloned in general, `JourneyResult` is not `Clone`.
- **`AbortReason`** has the reasons Rust can reach, `StepCouldNotBeBuilt`, `RequiredDataMissing`, `WrongType`, `PolicyCouldNotBeBuilt`, `HookFailed` and `ReporterFailed`, and is `#[non_exhaustive]`. `invalid lifecycle` and `not a value` cannot happen.
- **`Refusal`** names what refused the journey (a workflow policy, the dispatcher factory, or the dispatcher) and carries its `Error`.
- **`Violations`** (from `build()`), **`InstanceError`** (from `create()`) and **`Refusal`** implement `std::error::Error`.

## Testing

### 11. The conformance runner

- **A binary**, `itinera-conformance`, run with `cargo run -p itinera-conformance --release`, using cucumber-rs on Tokio (a dependency of the runner only). It reads the cases from `ITINERA_CONFORMANCE_CASES`, selects scenarios with the tag expression in `ITINERA_CONFORMANCE_TAGS`, writes Cucumber JSON to `ITINERA_CONFORMANCE_REPORT`, and fails if any selected scenario fails or uses an undefined sentence.
- **Only the public API.** Each scenario's sentences fill a scenario model, which "When the workflow runs" turns into builder calls: scripted steps implementing `StepFactory`, scripted policy factories recording what their hooks received, a scripted workflow type implementing one recording role trait, scripted reporters, and a recording dispatcher factory whose dispatchers share the test's reporter. Every sentence about events, and about step statuses, reads from that reporter. Scripted failures (a hook, role operation, reporter, dispatcher, factory or policy that fails) return an `Error`.
- **Both executors.** Every scenario whose workflow is `Sync` runs under `LocalExecutor` and again under `AsyncLocalExecutor`, and must pass under both. Scenarios with an asynchronous part run under the asynchronous one.
- **The runner knows nothing of excluded scenarios.** It writes only the Cucumber JSON, holding the scenarios that ran.
- **Exclusions are declared in `conformance.json`.** `impossible` maps each excluded tag to its scenarios, each with its feature file, its name, and its proof: the test that proves it and the file holding that test. Every scenario carrying the tag at the pinned cases must have an entry, so a tag is added, with all its entries, in the pull request that lands the proofs for all its scenarios. Until then its scenarios do not run anyway, since their proposals are not listed yet.
- **Proofs are Rust tests, run by `cargo test`.** They live in `crates/itinera/tests/proofs.rs`, with their fixtures beside it in `crates/itinera/tests/proofs/<tag>/`, because they test the public API, not the runner. Each excluded scenario has its own `#[test]`, which runs `trybuild` on one fixture holding the forbidden code, compared with the compiler error it must produce, and on a twin that differs only in the forbidden line and must compile. The fixtures share a few support types. The proofs are:
  - `invalid-lifecycle`: `on step failure` returning `FinishWorkflow`;
  - `role-not-provided`: a policy needing a role the workflow does not provide;
  - `mode-not-accepted`: an asynchronous step run by `LocalExecutor`;
  - `non-value`: a closure as initial data, as a contribution, as an adapter's value, as event data and as a reason's details;
  - `late-handle`: a contributor or step reporter moved into a thread that outlives the attempt.
- **The `run-conformance` action** checks the exclusions and writes the report's second file. It leaves the `impossible` tags out of the tag expression, fails if a tagged scenario has no entry or an entry names no scenario at the pinned cases, and writes the exclusions file next to the Cucumber JSON. Together the two files are the conformance report a release carries, written to `conformance-report/cucumber.json` and `conformance-report/exclusions.json` and uploaded as an artifact. While `conformance.json` lists no proposal, the action still checks the manifest but does not run the runner, so its job is part of CI from stage 0.
- **What runs.** A scenario runs once every proposal it is tagged with is listed in `conformance.json`. Listing a proposal therefore runs those of its scenarios whose other proposals are already listed; the rest join in, by themselves, when their last proposal is listed.
- **The tag expression** the action passes in `ITINERA_CONFORMANCE_TAGS` uses `and`, `or`, `not` and parentheses, for example `(@proposal-0002 or @proposal-0008) and not @non-value`. The runner parses it with cucumber-rs's tag expressions, which stage 2 tests.

### 12. Rust's own tests

- **Unit tests** in `itinera-core`, next to the code: the decision rules, typed reads from the bag, the order of events, the handling of `Interrupted`.
- **Macro equivalence**: each macro form is paired with the same workflow written with the builder. Both must give the same listing, and the same event stream for the same inputs, apart from the journey ID and timestamps.
- **Compile-fail tests** in `crates/itinera/tests/ui/`, each with a compiling twin: running an instance twice, calling `run` concurrently, misused macro attributes, a hook request its kind may not make, a handle field without its lifetime, an `async fn` step without the `async` feature.
- **Property-based tests** with `proptest`, over generated workflows with random outcomes, budgets, lifecycles and failing hooks or reporters: the first and last events, increasing sequence numbers, one outcome fact per attempt, one decision after each failed attempt, at most the budget plus one attempts, no bag values in engine events, and identical streams from both executors.
- **Doc tests**: every public item has an example that compiles and runs.
- **No `unsafe`**: `#![forbid(unsafe_code)]` in every crate.
- Coverage may be reported for information; it never blocks a merge.

### 13. Continuous integration

Each job runs one command, as the organisation's rules require. All are required to merge, except the weekly beta run.

| Check | Command |
|---|---|
| format | `cargo fmt --all --check` |
| lints | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| tests, on Linux, Windows and macOS | `cargo test --workspace --all-features` |
| sync-only build | `cargo check --workspace --no-default-features` |
| minimum Rust version | `cargo +1.85 check --workspace` |
| documentation | `cargo doc --workspace --no-deps`, with `RUSTDOCFLAGS=-D warnings` |
| dependencies | `cargo deny check`, through `EmbarkStudios/cargo-deny-action` |
| conformance, from stage 0 | `itinera-dev/actions/run-conformance@v1`, with `command: cargo run -p itinera-conformance --release`, after checking out and restoring the Rust cache |
| pull request rules | `pr-has-issue` and `no-cross-repo-closing` |
| weekly, not blocking | the tests on Rust beta |

The toolchain comes from `rust-toolchain.toml`; caching uses `Swatinem/rust-cache`; third-party actions are pinned to a commit.

## Order of work

### 14. Stages

| Stage | Content | Proposals completed |
|---|---|---|
| 0. Bootstrap | the workspace and crates, `rust-toolchain.toml`, lints, CI, `deny.toml`, the README, an agents' manual, `conformance.json` pinning the latest cases candidate, with the capabilities, no proposals, and `"impossible": {}` | none |
| 1. Events | `Error`, `Value` and `AnyValue` (the data bag waits for stage 5), `Event` and the types it carries, reporters, dispatchers and their factories, `DefaultDispatcher` | none |
| 2. Runner skeleton | cucumber-rs, the environment variables, the recording dispatcher factory, the scenario model, the Cucumber JSON report | none |
| 3. Minimal executor | the engine and both executors, `WorkflowDescriptor` with one synchronous step, `WorkflowInstance` and `Instance<W>`, journey IDs, reporters built by `create()`, `JourneyResult` and `Refusal` | none |
| 4. Declarations and admission | step and policy descriptors, input adapter declarations, the listing, `Violations` | 0062 |
| 5. Steps | needs and tokens, building per attempt, outcomes, contributions, the data bag, read-only received data, values, handles and `Interrupted` | 0056, 0057, 0064 (their scenarios are all proven impossible) |
| 6. The scan and its decisions | statuses, retries, `abnormal termination retriable`, aborts and the result; the decision logic with its hook points in place, tested through internal test hooks | 0012, 0032, 0042, 0061, 0063, 0065 |
| 7. Policies, hooks and roles | policy descriptors and factories, every hook kind and its requests, lifecycles, roles, workflow hooks, input adapters as hooks | 0002, 0008, 0009, 0010, 0011, 0024, 0027, 0040, 0041, 0049, 0054, 0055, 0058, 0060, 0081, 0083, 0085 |
| 8. Macros | `#[step]`, `#[step_policy]`, `#[workflow_policy]`, `#[workflow]`, their equivalence and compile-fail tests | none |
| 9. Release | the release workflow, `release-gate`, `0.1.0-rc.1` | none |

- **The decision logic is pure**: a function of the outcome, the retry budget left, `abnormal termination retriable` and the hooks' answers, which touches nothing. The code around it builds, runs, emits and commits.
- **Proofs land with their feature**, together with their tag's entries under `impossible` in `conformance.json`. A proposal is listed in `conformance.json` only when each of its scenarios that runs once it is listed passes, or is proven.
- **A later cases candidate** is adopted by a pull request that moves `cases` in `conformance.json` and fixes whatever its changed cases need.

### Pull requests

- Each stage is one stack (`gh stack`), with one layer per coherent piece. Every layer passes all of `main`'s checks.
- Every layer says `Refs #N` for the implementation issues it contributes to. The layer that completes a proposal says `Closes #N` and adds the proposal's number to `conformance.json`.
- Work that belongs to no proposal (stage 0, the macros, the release) refers to #7, or to an issue of its own such as "Macros as syntax over the builder".
- Public API of a proposal not yet listed in `conformance.json` stays behind the `unstable` feature. Its modules always compile, since the engine needs them; without `unstable` they are private.

## Issues and tech specs

### 15. Implementation issues

- **One issue per tier 1 proposal**, titled "Implement spec#NNNN (short name)", labelled `implements-proposal` and `tier: 1`. Its body links the proposal and its cases, summarises the decisions of this plan that concern it, lists its excluded scenarios and their proofs, and says it closes only when its cases pass.
- **[#2](https://github.com/itinera-dev/itinera-rs/issues/2)** is the implementation issue of 0002. Its comments are kept as history; the plan supersedes them.
- **Spec updates.** Once a proposal is listed in `conformance.json`, a later change to its cases gets a `spec update` issue, as PROCESS.md describes.
- **Tech specs**, `docs/specs/NNNN-short-name.md`, hold what is specific to one proposal: its API, how each of its rules is enforced (by types, by `build()`, or while running), its excluded scenarios and proofs, its own tests, and its definition of done. A tech spec is created in the first pull request that touches its proposal and completed in the one that closes it.

## Code

### 16. Style, structure and comments

- **Formatting**: plain `rustfmt`, without configuration.
- **Names** use the specification's vocabulary exactly: `JourneyId`, `StepDescriptor`, `Contributor`, `Outcome`, `retry_budget`, `FailWorkflow`. Event kinds keep their snake_case names.
- **No panics in library code**: Clippy denies `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `todo` and `unimplemented` outside tests.
- **A small public surface**: `pub(crate)` by default, checked by `unreachable_pub`; public types that may grow are `#[non_exhaustive]`.
- **Modules of `itinera-core` follow concepts**, named with the specification's vocabulary: `error`, `value`, `journey` (the journey ID, the journey's data and how it ends, the result included), `step`, `policy`, `event` (events, and what only events carry), `report` (reporters and dispatchers), `workflow` (descriptors, input adapters and violations), `instance`, `executor`, and the private `engine`. A type lives with its concept, not where it is first used, and modules may use one another. A module with submodules re-exports its public face, so public paths have one module level, such as `itinera::journey::JourneyId`; the crate root exports modules, not types.
- **Comments are as few as possible.** They explain a non-obvious reason only when the code cannot. Code and comments never refer to specification sections, issues, pull requests or other documents: git keeps that history, and the tech specs map rules to code. There are no `TODO` comments.
- **Public documentation** describes each item's behaviour in its own words, with an example. `missing_docs` is an error.
- **Test names** state the rule as a sentence, for example `a_failed_attempts_contributions_are_never_committed`.
- **British spelling** in documentation, as in the specification.

## Tooling

### 17. Toolchain, dependencies, features and documentation

- **`rust-toolchain.toml`** pins Rust 1.98.1 with `rustfmt` and `clippy`. Moving to a newer version is a pull request of its own, which also refreshes the stored compiler messages.
- **The minimum supported Rust version is 1.85**, declared as `rust-version`. Before 1.0, raising it is allowed in a minor release, and the changelog says so.
- **Dependencies**, each with default features off:

  | Crate | Used by | For |
  |---|---|---|
  | `serde`, `erased-serde` | core | the value bound, letting reporters serialize type-erased values |
  | `derive_more` (`display`, `from`, `into`, `as_ref`) | core | `Display`, `From`, `Into` and `AsRef` on newtypes and names, which the types use throughout |
  | `uuid` (`v4`) | core | the default journey ID |
  | `syn`, `quote`, `proc-macro2` | macros | the macros |
  | `cucumber` (`output-json`), `tokio`, `serde_json` | conformance runner | running the cases |
  | `trybuild`, `proptest`, `serde_json` | tests | compile-fail, property-based and equivalence tests |

  No async runtime and no `futures` crate in the core. A new dependency of the core needs a stated reason in its pull request.
- **`deny.toml`** allows the licences MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0 and Zlib, denies security advisories, accepts crates only from crates.io, and warns on duplicate versions.
- **Features of `itinera`** (forwarded to `itinera-core`), all additive:

  | Feature | Default | Adds |
  |---|---|---|
  | `macros` | on | the macros |
  | `async` | off | the asynchronous executor, steps, hooks, reporters and dispatchers, and the `Async` mode |
  | `unstable` | off | the API of proposals not yet listed in `conformance.json` |

  The engine is asynchronous internally whatever the features; without `async`, nothing asynchronous is public.
- **Documentation**: rustdoc on every public item, published by docs.rs; a crate-level overview in `itinera` with a first complete workflow, linking once to the specification repository; runnable examples in `crates/itinera/examples/` (a synchronous workflow, the same on the asynchronous executor, and one built without macros); a README stating the crates, the tier and capabilities claimed, the rules made impossible, and how to run the conformance runner.

## Release

### 18. Releasing the crates

- **Crates**: `itinera-core`, `itinera-macros` and `itinera`, with one workspace version, published together by `cargo publish --workspace`.
- **Versions are independent of the specification's.** The first release is `0.1.0-rc.1`. The README and the crate documentation say what each release implements: "specification 0.1.0, tier 1, capabilities `sync` and `async`, cases `v0.1.0-rc.N`".
- **The path to 0.1.0**:
  1. while the specification is in release candidates, Rust releases `0.1.0-rc.N`, pinning the latest cases candidate;
  2. once Rust, the first implementation, passes every tier 1 case, the specification and conformance are tagged `v0.1.0`;
  3. Rust then releases `0.1.0`, pinning cases `v0.1.0`.
- **A release is requested by a maintainer pushing a tag `v*` on `main`**; a repository ruleset restricts those tags to maintainers. The release workflow runs four jobs:
  1. **gate**: the `release-gate` action checks that the tag matches the workspace version, that the commit is on `main`, and that `conformance.json` lists every proposal of the tier claimed, taken from the `@proposal-NNNN` tags of the cases at the pinned version;
  2. **conformance**: `run-conformance`, producing the report;
  3. **publish**: in the `crates-io` environment, which needs a maintainer's approval, authenticated through trusted publishing with `rust-lang/crates-io-auth-action`, then `cargo publish --workspace`;
  4. **release**: `gh release create`, a pre-release for `-rc` tags, with the conformance report attached.
- **The first publish of each crate.** crates.io configures trusted publishing only for crates that already exist, and PROCESS.md leaves such registry exceptions to each language. Ours: the first release of each crate is published by the release workflow, after its gate, with a short-lived token that a maintainer creates for that release only and stores as a secret of the `crates-io` environment. The token is deleted as soon as the release is published and trusted publishing is configured for the crates; every later release uses trusted publishing only. Nothing is published before that first release: crate names are not reserved with placeholder packages.
- **`CHANGELOG.md`** is updated in the pull request that bumps the version, and the release notes come from it.
- **`release-gate`** is written in itinera-dev/actions, in Python with tests.

## Also

### 19. The rest

- **An agents' manual** in this repository (`AGENTS.md`, imported by `CLAUDE.md`), self-contained: the rule that behaviour comes only from the specification, the stages, the comment rules, how to run the checks and the runner, and stacks.
- **The README** is updated in stage 0 to the crates and status of this plan.
- **Versioning before 1.0**: a breaking change bumps the minor version and a fix the patch version. The changelog states the specification version and cases each release implements.
- **Security reporting**: stage 0 checks that the organisation's community files include a security policy covering this repository.
- **Out of scope for tier 1**: cooperative yields in the async engine, integration crates such as a `tracing` reporter, benchmarks, and durable executors.

### Notes for other languages

Points that came up while planning Rust, which Java, TypeScript or other implementations will meet in their own tech specs:

- A step's reporter is best given as a small per-attempt object that implements the reporter interface and holds the dispatcher privately: handing over the dispatcher itself would let a step cast it back. The executor closes it when its attempt ends.
- Where reporters throw, the executor's wrapper records the abort before throwing into the step, so a step that catches everything still cannot keep its journey alive; the exception type is best left unexported.
- Input adapters can be found by reflection in a base class of the workflow, which dispatches each request to the annotated method.
- Where the language has no read-only receiver, rebuilding policies for every attempt and every journey is what keeps hooks from carrying state.
