# Tier 1 plan

How itinera-rs implements tier 1 of specification 0.1.0: the architecture, the tests, the order of work, the issues, the tooling and the release. Tracked in [#7](https://github.com/itinera-dev/itinera-rs/issues/7).

This document holds what crosses proposals. What concerns a single proposal goes in its tech spec, `docs/specs/NNNN-short-name.md`, which refers here for the rest. Behaviour is defined only by the [specification](https://github.com/itinera-dev/spec/tree/main/spec): where this plan and the specification disagree, the specification is right, and the plan is corrected.

Code samples show the intended shape. Names of attributes and methods may still change while implementing; the tech specs record the final ones.

## Open specification items

Parts of this plan rely on changes still open in [itinera-dev/spec](https://github.com/itinera-dev/spec). Implementation issues are opened, and stage 3 starts, only once a maintainer says they are settled. If one is declined or changed, the decisions that depend on it are revisited before code is written.

| Issue | What it settles | Decisions that depend on it |
|---|---|---|
| [#54](https://github.com/itinera-dev/spec/issues/54) | Rules a language makes impossible to express are excluded from conformance, each with a proof; unrecoverable failures, such as a panic, are outside the model | 2, 4, 5, 8, 11 |
| [#55](https://github.com/itinera-dev/spec/issues/55) | Events from steps and hooks are delivered before the emit call returns; a reporter that throws while a step runs ends it | 8, 9 |
| [#56](https://github.com/itinera-dev/spec/issues/56) | Everything in the data bag is a serializable value | 3 |
| [#57](https://github.com/itinera-dev/spec/issues/57) | Contributors and reporters are valid only during their attempt or hook | 4 |
| [#58](https://github.com/itinera-dev/spec/issues/58) | Workflow policies are built per journey, step policies per attempt; hooks cannot change their policy | 5 |
| [#59](https://github.com/itinera-dev/spec/issues/59) | A name for the violation "input adapter for an unknown step or key" | 6 |
| [#60](https://github.com/itinera-dev/spec/issues/60) | Input adapters are hooks attached to whole steps, and leave unknown inputs to the data bag | 6 |
| [#61](https://github.com/itinera-dev/spec/issues/61) | The workflow instance provides its journey ID and its reporters | 7, 8 |
| [#62](https://github.com/itinera-dev/spec/issues/62) | Workflow descriptors, and the workflow instance interface | 6, 7 |
| [#63](https://github.com/itinera-dev/spec/issues/63) | Dispatchers gain `clear` | 9 |
| [#64](https://github.com/itinera-dev/spec/issues/64) | Event data and reason details are values carried in memory, serialized only by reporters | 9 |
| [#65](https://github.com/itinera-dev/spec/issues/65) | The journey result is a business outcome, without step internals | 10 |
| [#66](https://github.com/itinera-dev/spec/issues/66) | The first publish of a package uses a short-lived token | 18 |

In itinera-dev/conformance, [#37](https://github.com/itinera-dev/conformance/issues/37) settles how JSON values map to the neutral types.

## Architecture

### 1. Crates

A Cargo workspace, edition 2024, resolver 3, under `crates/`.

| Crate | Published | Holds |
|---|---|---|
| `itinera` | yes | What applications depend on: re-exports `itinera-core`, and the macros behind the `macros` feature |
| `itinera-core` | yes | Values and the data bag, events, reporters and dispatchers, steps, policies, descriptors and their builder, the workflow instance, the engine and both local executors |
| `itinera-macros` | yes | The procedural macros |
| `itinera-conformance` | no | The conformance runner and its proofs |

There is no separate executor crate in tier 1: both executors share one engine, which stays private. Executors of later tiers, such as durable ones, get crates of their own.

### 2. A typed builder, and rules made impossible to express

The builder is the public API, and the macros are only syntax over it. There is no untyped API. What Rust can reject at compile time, it rejects:

- **Lifecycles.** Each hook has its own return type, holding only the lifecycles it may return, so an invalid lifecycle cannot be written.
- **Roles.** A workflow is generic over its own type `W`, and a policy that needs a role is implemented only for workflows whose `W` implements that role's trait. Attaching it to a workflow without the role does not compile.
- **Execution modes.** The mode is part of the descriptor's type. Adding an asynchronous part turns a `Sync` descriptor into an `Async` one, and the synchronous executor accepts only `Sync`.

The conformance scenarios that need these situations are excluded through the tags of spec#54, and each is replaced by a proof (decision 11). The rules types cannot reach are checked when the descriptor is built (decision 6) or while the journey runs.

### 3. Values and the data bag

- **A value** is any `T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static`. Closures, function pointers and handles cannot be values. Nothing is serialized by the engine: the bounds only make sure that something could serialize them.
- **The data bag** maps `String` keys to type-erased values, together with what is needed to clone and serialize them.
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
      fn run(self) -> Result<Outcome, Error> { /* ... */ }
  }
  ```

- **`run(self)` takes nothing else.** It consumes the step, so nothing survives into another attempt. `Step` and `AsyncStep` are separate traits, so the mode is in the type.
- **Handles borrow their attempt.** `Contributor<'a>` and `StepReporter<'a>` carry the lifetime of the attempt, chosen by the executor, so they cannot be moved into a thread or task that outlives it. The guarantee comes from the types, with or without the macros.
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
- **One error type, `itinera::Error`**, for the custom code that can fail: a step's `run`, a step's constructor, and a reporter's `init`. It converts from any `std::error::Error + Send + Sync + 'static`, so `?` works on any library's error. Because of that conversion it does not itself implement `std::error::Error`; it offers `Display`, `source()` and `Error::msg`.
- **`?` in `run` is an abnormal termination**: an error the step did not anticipate. A failure the step chose is returned as `Outcome::failure`.
- **Contributions are kept per attempt**: visible to that attempt's hooks whatever its outcome, committed only on Success, dropped on an abnormal termination.

### 5. Policies, hooks and roles

- **Two kinds of policy**, `StepPolicy<W>` and `WorkflowPolicy<W>`. A policy type can hold only hooks of its own kind. Each policy has a name, which defaults to its type's name with the macros and is required with the builder.
- **Hooks declare what they need like steps do**, as annotated parameters. What a hook may request depends on its kind, so a request it may not make does not compile:

  | Request | Available to |
  |---|---|
  | data from the step, the step's name, the attempt number | step hooks |
  | what failed: a reported `Reason`, or the `Error` of an abnormal termination | `on step failure`, `on step retry` |
  | the cause, as its own enum per hook | `on step failure`, `on step retry` |
  | the error | `on step abnormal termination` |
  | data from the workflow, the journey ID, roles, a contributor, a reporter | every hook |

- **Each hook returns its own type**: `Option<OnSuccess>` for `on step success` (`FinishWorkflow` or `FailWorkflow`), `Option<FailWorkflow>` for `on step failure`, `on step retry` and `on step abnormal termination`, and nothing for workflow hooks.
- **Hooks take `&self`.** Everything they need arrives as parameters, so they have no reason to change their policy.
- **Policies are built by the executor from factories**, which cannot fail: workflow policies once per journey, before it starts; step policies for every attempt, with the step.
- **Roles are plain traits**, implemented by the workflow's own type `W`. A hook requests one as `#[role] notifier: &dyn Notifier`. There is no marker trait. Role operations return no `Result`.
- **Hooks, role operations, reporters and dispatchers cannot fail**: their signatures have no error. A panic is a bug, is never caught, and ends the program as Rust defines.
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
- **`build()` checks what types cannot**, and returns every violation at once in `Violations`: duplicate step names, a hook defined twice on one step or on the workflow, an input adapter for an unknown step, and a step adapted twice.
- **`listing()`** gives each step's name, its position from 1, its policy names in order, and its input adapter's name.
- **Input adapters are hooks of the workflow**, written as its own annotated methods and attached to whole steps, at most one per step:

  ```rust
  #[workflow(name = "orders")]
  impl Orders {
      #[input_adapter(steps = ["charge", "refund"])]
      fn pricing(&self, #[step_name] step: &str, #[key] key: &str,
                 #[data_from_workflow("price")] price: i64) -> Option<Value> { /* ... */ }
  }
  ```

  It is called for each input of an adapted step. A value is used; `None` means "not mine", and the input is read from the data bag. Its return is untyped and checked against the step's declared type. It may request the step's name, the key, data from the workflow and the journey ID, never roles, a contributor or a reporter. The macro generates the builder call that registers it.

### 7. The workflow instance

Every workflow instance implements one trait, the only thing an executor relies on. The executor coordinates the flow; it does not own data.

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
- **`run` returns the journey's result directly**, with no `Result`, because every refusal the specification defines is settled before `run`: declaration violations by `build()`, modes by types, the journey ID and reporters by `create()`, and dispatchers that cannot fail.

  ```rust
  impl<D: Dispatcher> LocalExecutor<D> {
      pub fn new() -> LocalExecutor<DefaultDispatcher>;
      pub fn with_dispatcher(dispatcher: D) -> Self;
      pub fn run<I: WorkflowInstance<Mode = Sync>>(&mut self, instance: I) -> JourneyResult;
  }
  ```

  `&mut self` makes concurrent journeys on one executor impossible to write.
- **Everything asynchronous is awaited**: asynchronous steps, hooks, reporters and dispatchers alike.
- **A dropped future abandons the journey.** If the caller drops the future returned by `AsyncLocalExecutor::run`, the journey stops where it is. The documentation says so.
- **Deferred:** cooperative yields between units of the async engine.

### 9. Events and dispatchers

- **One `Event` type**: a sequence number from 1, a `SystemTime` timestamp rendered as ISO 8601 in UTC, the journey ID, the workflow name, and a typed body with one variant per event of the catalogue. `kind()` returns the event's snake_case name. No variant has a field able to hold a value from the data bag. `DecidedBy` is `Default` or a policy and hook. `Event` implements `Serialize`.
- **Data in step and hook events, and reason details,** are values carried in memory, cloned when emitted, and serialized only by reporters, in their own format.
- **Reporters**: `Reporter` with `report(&mut self, &Event)`, and `AsyncReporter` behind the `async` feature. An asynchronous reporter makes the workflow `Async`.
- **The handles given to steps and hooks** are restricted views of the journey's dispatcher. A `StepReporter` emits only `step_info`, `step_warning` and `step_error`, stamped with the step and attempt; a `HookReporter` emits only `journey_info`, `journey_warning` and `journey_error`, stamped with the policy and hook. Delivery happens before the emit call returns. In a workflow with an asynchronous reporter, the handles are asynchronous, and only asynchronous steps and hooks can request them.
- **Dispatchers**: `Dispatcher` and `AsyncDispatcher`, with `add`, `dispatch` and `clear`. `clear` removes every reporter added through `add` and keeps the dispatcher's own. `DefaultDispatcher` implements both, is created afresh for every journey, and calls reporters in the order they were added. A dispatcher given to an executor is kept between journeys and cleared at the end of each.
- **One emitter** in the engine assigns sequence numbers and timestamps. Its clock can be replaced inside the crate for tests.

### 10. The result and the errors

```rust
pub struct JourneyResult {
    pub journey_id: JourneyId,
    pub status: JourneyStatus,
    pub data: DataBag,
}

pub enum JourneyStatus { Succeeded, Failed(Failure), Aborted(Abort) }

pub enum Failure {
    Failed(Reason),
    RetriesExhausted(Reason),
    AbnormalTermination(String),
    FailedByPolicy(Reason),
}

pub struct Abort { pub reason: AbortReason, pub details: AbortDetails }
```

- **The result is a business outcome.** It holds no step statuses, attempt counts or step names: those are observable in the event stream.
- **`AbortReason`** has the reasons Rust can reach, `StepCouldNotBeBuilt`, `RequiredDataMissing` and `WrongType`, and is `#[non_exhaustive]`.
- **`Violations`** (from `build()`) and **`InstanceError`** (from `create()`) implement `std::error::Error`.

## Testing

### 11. The conformance runner

- **A binary**, `itinera-conformance`, run with `cargo run -p itinera-conformance --release`, using cucumber-rs on Tokio (a dependency of the runner only). It reads the cases from `ITINERA_CONFORMANCE_CASES`, selects scenarios with the tag expression in `ITINERA_CONFORMANCE_TAGS`, writes Cucumber JSON to `ITINERA_CONFORMANCE_REPORT`, and fails if any selected scenario fails or uses an undefined sentence.
- **Only the public API.** Each scenario's sentences fill a scenario model, which "When the workflow runs" turns into builder calls: scripted steps implementing `StepFactory`, scripted policy factories recording what their hooks received, a scripted workflow type implementing one recording role trait, and a recording dispatcher holding the test's reporter. Every sentence about events, and about step statuses, reads from that dispatcher.
- **Both executors.** Every scenario whose workflow is `Sync` runs under `LocalExecutor` and again under `AsyncLocalExecutor`, and must pass under both. Scenarios with an asynchronous part run under the asynchronous one.
- **Proofs.** Every scenario carrying a tag listed under `impossible` in `conformance.json` has a fixture in `crates/itinera-conformance/proofs/<tag>/`: a short file with the forbidden code, the compiler error it must produce, and a twin that differs only in the forbidden line and must compile. `proofs.toml` maps each scenario to its fixture, and the runner fails if a tagged scenario has none. The fixtures use shared support types from the conformance crate. Tier 1 needs proofs for:
  - an asynchronous step run by `LocalExecutor`;
  - `on step failure` returning `FinishWorkflow`;
  - a policy needing a role the workflow does not provide;
  - a hook or role operation that can fail;
  - a reporter or dispatcher that can fail;
  - a non-value in the data bag;
  - a handle used after its attempt;
  - a policy factory that can fail;
  - event data that is not serializable.
- **The `run-conformance` action** is written in itinera-dev/actions, in Python with tests, before the first proposal is listed in `conformance.json`.

### 12. Rust's own tests

- **Unit tests** in `itinera-core`, next to the code: the decision rules, typed reads from the bag, the order of events, dispatchers' `clear`.
- **Macro equivalence**: each macro form is paired with the same workflow written with the builder. Both must give the same listing, and the same event stream for the same inputs, apart from the journey ID and timestamps.
- **Compile-fail tests** in `crates/itinera/tests/ui/`, each with a compiling twin: running an instance twice, calling `run` concurrently, misused macro attributes, a hook request its kind may not make, a handle field without its lifetime, an `async fn` step without the `async` feature.
- **Property-based tests** with `proptest`, over generated workflows with random outcomes, budgets and lifecycles: the first and last events, increasing sequence numbers, one outcome fact per attempt, one decision after each failed attempt, at most the budget plus one attempts, no bag values in engine events, and identical streams from both executors.
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
| conformance | `itinera-dev/actions/run-conformance@v1` |
| pull request rules | `pr-has-issue` and `no-cross-repo-closing` |
| weekly, not blocking | the tests on Rust beta |

The toolchain comes from `rust-toolchain.toml`; caching uses `Swatinem/rust-cache`; third-party actions are pinned to a commit.

## Order of work

### 14. Stages

| Stage | Content | Proposals completed |
|---|---|---|
| 0. Bootstrap | the workspace and crates, `rust-toolchain.toml`, lints, CI, `deny.toml`, the README, an agents' manual, `conformance.json` pinning the latest cases candidate with no proposals | none |
| 1. Events | `Event`, reporters, dispatchers with `clear`, `DefaultDispatcher` | none |
| 2. Runner skeleton | cucumber-rs, the environment variables, the recording dispatcher, the scenario model, the proofs mechanism | none |
| 3. Minimal executor | the engine and both executors, `WorkflowDescriptor` with one synchronous step, `WorkflowInstance` and `Instance<W>`, journey IDs, reporters built by `create()`, `JourneyResult` | none |
| 4. Declarations and admission | step and policy descriptors, input adapters, the listing, `Violations` | none |
| 5. Steps | needs and tokens, building per attempt, outcomes, contributions, the data bag, read-only received data | none |
| 6. The scan and its decisions | statuses, retries, `abnormal termination retriable`, aborts and the result; the decision logic with its hook points in place, tested through internal test hooks | 0012, 0032, 0042 |
| 7. Policies, hooks and roles | policy descriptors and factories, every hook kind and its requests, lifecycles, roles, workflow hooks, input adapters as hooks | 0002, 0008, 0009, 0010, 0011, 0024, 0027, 0040, 0041, 0049, and the accepted amendments |
| 8. Macros | `#[step]`, `#[step_policy]`, `#[workflow_policy]`, `#[workflow]`, their equivalence and compile-fail tests | none |
| 9. Release | the release workflow, `release-gate`, `0.1.0-rc.1` | none |

- **The decision logic is pure**: a function of the outcome, the retry budget left, `abnormal termination retriable` and the hooks' answers, which touches nothing. The code around it builds, runs, emits and commits.
- **Proofs land with their feature**, for example the invalid-lifecycle proof in stage 7. A proposal is listed in `conformance.json` only when each of its scenarios passes or is proven.
- **Stages 0 to 2 can start before** the open specification items are settled; stage 3 waits for #54, #61 and #62.

### Pull requests

- Each stage is one stack (`gh stack`), with one layer per coherent piece. Every layer passes all of `main`'s checks.
- Every layer says `Refs #N` for the implementation issues it contributes to. The layer that completes a proposal says `Closes #N` and adds the proposal's number to `conformance.json`.
- Work that belongs to no proposal (stage 0, the macros, the release) refers to #7, or to an issue of its own such as "Macros as syntax over the builder".
- Public API of a proposal not yet listed in `conformance.json` stays behind the `unstable` feature.

## Issues and tech specs

### 15. Implementation issues

- **One issue per accepted tier 1 proposal**, titled "Implement spec#NNNN (short name)", labelled `implements-proposal` and `tier: 1`. Its body links the proposal and its cases, summarises the decisions of this plan that concern it, lists its excluded scenarios and their proofs, and says it closes only when its cases pass.
- **The open proposals of the table above** that are later accepted get their issues the same way; they are tier 1 amendments of specification 0.1.
- **Spec defects get no issue** while nothing is implemented. Once a proposal is listed in `conformance.json`, a later change to its cases gets a `spec update` issue, as PROCESS.md describes.
- **[#2](https://github.com/itinera-dev/itinera-rs/issues/2)** becomes the implementation issue of 0002. Its body is rewritten in the same form and points here; its comments stay as history.
- **Tech specs**, `docs/specs/NNNN-short-name.md`, hold what is specific to one proposal: its API, how each of its rules is enforced (by types, by `build()`, or while running), its excluded scenarios and proofs, its own tests, and its definition of done. A tech spec is created in the first pull request that touches its proposal and completed in the one that closes it.

## Code

### 16. Style, structure and comments

- **Formatting**: plain `rustfmt`, without configuration.
- **Names** use the specification's vocabulary exactly: `JourneyId`, `StepDescriptor`, `Contributor`, `Outcome`, `retry_budget`, `FailWorkflow`. Event kinds keep their snake_case names.
- **No panics in library code**: Clippy denies `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `todo` and `unimplemented` outside tests.
- **A small public surface**: `pub(crate)` by default, checked by `unreachable_pub`; public types that may grow are `#[non_exhaustive]`.
- **Modules of `itinera-core`**, each depending only on those before it: `error`, `value`, `event`, `report`, `step`, `policy`, `descriptor`, `instance`, `result`, `engine` (private), `executor`.
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
  | `serde`, `erased-serde` | core | the value bound, `Serialize` on events, letting reporters serialize type-erased values |
  | `uuid` (`v4`) | core | the default journey ID |
  | `time` (`formatting`) | core | ISO 8601 timestamps |
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
- **Documentation**: rustdoc on every public item, published by docs.rs; a crate-level overview in `itinera` with a first complete workflow, linking once to the specification repository; runnable examples in `crates/itinera/examples/` (a synchronous workflow, the same on the asynchronous executor, and one built without macros); a README stating the crates, the tier and capabilities claimed, and how to run the conformance runner.

## Release

### 18. Releasing the crates

- **Crates**: `itinera-core`, `itinera-macros` and `itinera`, with one workspace version, published together by `cargo publish --workspace`. Nothing is published before the first real release; names are not reserved.
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
- **The first publish** uses a short-lived token held by the `crates-io` environment, because crates.io configures trusted publishing only for crates that exist. The token is deleted right after, and every later release uses trusted publishing (spec#66).
- **`CHANGELOG.md`** is updated in the pull request that bumps the version, and the release notes come from it.
- **`release-gate`** is written in itinera-dev/actions, in Python with tests.

## Also

### 19. The rest

- **An agents' manual** in this repository (`AGENTS.md`, imported by `CLAUDE.md`), self-contained: the rule that behaviour comes only from the specification, the stages, the comment rules, how to run the checks and the runner, and stacks.
- **The label `spec update`** is created in this repository when the implementation issues are opened.
- **The README** is updated in stage 0 to the crates and status of this plan.
- **Versioning before 1.0**: a breaking change bumps the minor version and a fix the patch version. The changelog states the specification version and cases each release implements.
- **Security reporting**: stage 0 checks that the organisation's community files include a security policy covering this repository.
- **Out of scope for tier 1**: cooperative yields in the async engine, integration crates such as a `tracing` reporter, benchmarks, and durable executors.

### Notes for other languages

Points that came up while planning Rust, which Java, TypeScript or other implementations will meet in their own tech specs:

- A step's reporter is best given as a small per-attempt object that implements the reporter interface and holds the dispatcher privately: handing over the dispatcher itself would let a step cast it back. The object is closed when its attempt ends.
- Where reporters can throw, the executor's wrapper records the abort before throwing into the step, so a step that catches everything still cannot keep its journey alive; the exception type is best left unexported.
- Input adapters can be found by reflection in a base class of the workflow, which dispatches each request to the annotated method.
- Where the language has no read-only receiver, rebuilding policies for every attempt and every journey is what keeps hooks from carrying state.
