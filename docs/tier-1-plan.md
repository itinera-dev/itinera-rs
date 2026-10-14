# Tier 1 plan

How itinera-rs implements tier 1 of specification 0.1.0: the architecture, the tests, the order of work, the issues, the tooling and the release. Tracked in [#7](https://github.com/itinera-dev/itinera-rs/issues/7).

This document holds what crosses proposals. What concerns a single proposal goes in its tech spec, `docs/specs/NNNN-short-name.md`, which refers here for the rest. Behaviour is defined only by the [specification](https://github.com/itinera-dev/spec/tree/main/spec): where this plan and the specification disagree, the specification is right, and the plan is corrected.

Code samples show the intended shape. Names of attributes and methods may still change while implementing; the tech specs record the final ones, and section 11 those of the macros.

## What this plan implements

- **Specification 0.1.0, tier 1**: proposals 0002, 0008, 0009, 0010, 0011, 0012, 0024, 0027, 0032, 0040, 0041, 0042, 0049, 0054, 0055, 0056, 0057, 0058, 0060, 0061, 0062, 0063, 0064, 0065, 0081, 0083, 0085 and 0091, which amends 0010, 0054 and 0060; and 0003, rescue steps, 0095, services provided to steps, 0098, input adapters and service providers for rescue steps, 0102, global metadata, and 0104, the implementation a service provider made, which joined tier 1 later. Also 0108, test support, listed because Rust claims `testing`, though it counts towards no tier.
- **The cases** at [itinera-dev/conformance](https://github.com/itinera-dev/conformance) `v0.1.0-rc.11`, and later candidates as they are tagged.
- **Capabilities claimed**: `sync`, `async` and `testing`.
- **Rules made impossible to express** (proposal 0054): `invalid-lifecycle`, `role-not-provided`, `mode-not-accepted`, `non-value`, `late-handle`, `bag-write`, `rescue-skip`, `journey-scope-request` and `no-instance`.

How excluded scenarios are declared and reported is defined in the conformance repository's FORMAT.md: the manifest lists each one with its proof, and the `run-conformance` action reports them next to the Cucumber JSON (decision 11).

## Architecture

### 1. Crates

A Cargo workspace, edition 2024, resolver 3, under `crates/`.

| Crate | Published | Holds |
|---|---|---|
| `itinera` | yes | What applications depend on: re-exports `itinera-core`, the macros behind the `macros` feature, and `mode::async_trait` behind `async` |
| `itinera-core` | yes | Values and the data bag, events, reporters and dispatchers, steps, policies, descriptors and their builder, the workflow instance, the engine and both local executors |
| `itinera-macros` | yes | The procedural macros |
| `itinera-conformance` | no | The conformance runner |
| `xtask` | no | Development tasks: `cargo xtask check` runs every required check in order |

There is no separate executor crate in tier 1: both executors share one engine, which stays private. Executors of later tiers, such as durable ones, get crates of their own.

### 2. A typed builder, and rules made impossible to express

The builder is the public API, and the macros are only syntax over it. There is no untyped API. Six rules are made impossible to express, each proven by a test (decision 11):

- **Lifecycles.** Each hook has its own return type, holding only the lifecycles it may return, so an invalid lifecycle cannot be written.
- **Roles.** A workflow is generic over its own type `W`, and a policy that needs a role is implemented only for workflows whose `W` implements that role's trait. Attaching it to a workflow without the role does not compile.
- **Execution modes.** The mode is part of the descriptor's type, and a workflow is declared in it from the start: `WorkflowDescriptor::builder` declares a `Synchronous` workflow, and `WorkflowDescriptor::async_builder` an `Asynchronous` one. In an asynchronous workflow, everything that may wait on the journey is asynchronous: its steps and hooks are, and a step whose work is synchronous is written as an `async fn` that never awaits. Its reporters and dispatchers may be of either kind, since the executor wraps a synchronous one. The synchronous executor accepts only `Synchronous` workflows, and the asynchronous executor only `Asynchronous` ones. A synchronous step could not wait for an asynchronous delivery, which is why the builder does not infer the mode from the parts; the macros take it from the steps, as the section on macros says. The modes are `itinera::mode::Synchronous` and `itinera::mode::Asynchronous`, named so that they never hide the standard `Sync` trait.
- **Values.** Only values can enter the data bag, event data or a reason's details (decision 3).
- **Late handles.** A contributor or reporter cannot outlive its attempt or hook, nor an input adapter's access to the data bag its call (decision 4).
- **Writes through an adapter's access.** An input adapter's access to the data bag offers reads only, so a write through it cannot be written.

The rules types cannot reach are checked when the descriptor is built (decision 6) or while the journey runs.

### 3. Values and the data bag

- **A value** is any `T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static`. Closures, function pointers and handles cannot be values. Nothing is serialized by the engine: the bounds only make sure that something could serialize them.
- **The data bag** maps `String` keys to type-erased values, together with what is needed to clone and serialize them. Event data and reason details use the same type-erased value.
- **Reading** names the exact type `T`. Any other type is `wrong type`, including `i32` against `i64`.
- **Received data is read-only** because every receiver, step or hook, gets its own owned clone, never a reference into the bag. A type whose `Clone` shares mutable state, such as `Arc<Mutex<_>>`, defeats this, and the documentation says so.
- **Contributions are captured when made**: contributing takes the value by ownership.
- **The conformance runner** maps the neutral types to Rust types: `string` to `String`, `integer` to `i64`, `number` to `f64`, `boolean` to `bool`, `list` to `Vec<serde_json::Value>`, `object` to `serde_json::Map<String, Value>`.

### 4. Steps

- **A step declares everything it needs, and asks only for what it uses**: inputs, optional inputs, a `Contributor<'a>` and a `StepReporter<'a>`. With the macros (section 11), its inputs are the fields of its struct, or the parameters of its `new`, and its handles are parameters of `run`:

  ```rust
  #[derive(Step)]
  struct Charge {
      #[input]
      amount: i64,
      #[input("coupon")]
      discount: Option<i64>,
  }

  #[step]
  impl Charge {
      fn run(
          self,
          #[contributor] mut contributor: Contributor<'_>,
          #[reporter] mut reporter: StepReporter<'_>,
      ) -> Result<Outcome, Error> {
          reporter.info("charging")?;
          contributor.contribute("receipt", "R-1".to_owned());
          Self::succeed()
      }
  }
  ```

- **`Step::run(self)` takes nothing else.** It consumes the step, so nothing survives into another attempt. A factory hands the step its handles when it builds it; `#[step]` generates a hidden step that keeps them and passes them to the `run` written in the block. `Step` and `AsyncStep` are separate traits, so the mode is in the type.
- **Handles borrow their attempt.** `Contributor<'a>` and `StepReporter<'a>` carry the lifetime of the attempt, chosen by the executor, so they cannot be moved into a thread or task that outlives it. The guarantee comes from the types, with or without the macros.
- **Emitting can be interrupted.** `StepReporter` has `info`, `warning` and `error`, each with a `_with` form that adds data, a value; `AsyncStepReporter` has the same methods, awaited. They also have `set_global_metadata(key, value)`, which sets an entry of global metadata. All of them return `Result<(), Interrupted>`. `Interrupted` is the executor's signal that a reporter failed while the event was being delivered (decision 9); the step propagates it with `?`. `Contributor::contribute` returns nothing: it cannot fail, since only values can be contributed.
- **The builder takes a `StepFactory`**, which `#[step]` generates:

  ```rust
  pub trait StepFactory: Send + Sync + 'static {
      type Step<'a>: Step;
      fn needs(&self) -> StepNeeds;
      fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Self::Step<'a>, Error>;
  }
  ```

  Inputs are typed tokens, usually constants: `const AMOUNT: Input<i64> = Input::new("amount")` and `OptionalInput<T>`. `needs` lists them, as in `StepNeeds::new().input(&AMOUNT).optional_input(&DISCOUNT)`, and `build` takes each resolved value, already of its type, with `got.input(&AMOUNT)?` or `got.optional_input(&DISCOUNT)?`. Handles are declared and taken the same way, with `.contributor()` and `.reporter()`. The engine checks each value against the declared type before building; a token the step did not declare, or one taken twice, is an error from `build`, so the step could not be built. A step may borrow its factory, which lives as long as the attempt. A closure returning `Result<Outcome, Error>` is the factory of a step that needs nothing. The asynchronous traits, `AsyncStep` and `AsyncStepFactory`, mirror these, and `StepDescriptor::new_async` takes the factory.
- **Outcomes**: `Outcome::success()`, `Outcome::failure(reason)`, `Outcome::skipped()` and `Outcome::skipped_because(reason)`, read back with `is_success()`, `is_failure()`, `is_skipped()` and `reason()`, which `RescueOutcome` offers too, except `is_skipped()`. A `Reason` has a code, an optional message and optional details, which are a value.
- **One error type, `itinera::error::Error`**, for every custom code that can fail: a step's `run` and constructor, hooks, role operations, policy factories, reporters, as they report an event or receive global metadata, dispatchers and dispatcher factories. It converts from any `std::error::Error + Send + Sync + 'static`, so `?` works on any library's error, and from `Interrupted`. Because of that conversion it does not itself implement `std::error::Error`; it offers `Display`, `source()` and `Error::msg`.
- **`?` in `run` is an abnormal termination**: an error the step did not anticipate. A failure the step chose is returned as `Outcome::failure`. The one exception is `Interrupted`: the engine recognises its own signal, the journey is already aborted, and nothing the step returns afterwards counts.
- **Contributions are kept per attempt**: visible to that attempt's hooks whatever its outcome, committed only on Success, in the order their keys were first contributed, each key with its last value, and dropped otherwise.

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

- **Each hook returns its own type, inside a `Result`**: `Result<Option<OnSuccess>, Error>` for `on step success` (`FinishWorkflow` or `FailWorkflow`), `Result<Option<FailWorkflow>, Error>` for `on step failure`, `on step retry` and `on step abnormal termination`, and `Result<(), Error>` for workflow hooks. `Err` aborts the journey with `hook failed`. A test that calls a step hook itself reads its answer through the sealed trait `ReturnedLifecycle`, implemented for both options: `keeps_default()`, `fails_workflow()`, which gives the reason, and `finishes_workflow()`. `FailWorkflow::reason()` reads the reason of one `FailWorkflow`.
- **Hooks take `&self`.** Everything they need arrives as parameters, so they have no reason to change their policy.
- **The builder takes the hook traits, which the macros implement.** Each hook has a trait of its own, such as `OnStepSuccess<W>`, with `fn needs() -> HookNeeds<StepSuccess>` and the hook itself, which receives `Requested<'_, W, StepSuccess>`. `HookNeeds` declares the requests with the tokens steps use, through `from_step`, `from_workflow`, their optional forms, `reason`, `error`, their optional forms, `contributor` and `reporter`, and only those its kind allows compile. The hook takes each resolved value from `Requested`, with `got.from_step(&RECEIPT)?` and the like. The requests are resolved in the order declared, and the first required one without a value aborts the journey before the hook runs. The specification does not say in which order yet; spec#92 asks it to.
- **A policy descriptor names the hooks it defines.** `StepPolicyDescriptor::new(name, factory)`, or `fallible` for a factory that can fail, and their `_async` forms, return a `Hookless` descriptor. `.on_step_success()` and the other hooks of its kind make it `Hooked`, and `.on_step_success_needing(needs)` replaces what the hook's type declares for that descriptor. Only a `Hooked` descriptor can be attached, so a policy that defines no hook is not expressible. `WorkflowPolicyDescriptor` does the same for workflow hooks.
- **Policies are built by the executor from factories** that return the policy or an `Error`: workflow policies once per journey, before it starts, where an error is a refusal; step policies for every attempt, with the step, where an error aborts the journey with `policy could not be built`. The builder also accepts factories that cannot fail.
- **Roles are plain traits**, implemented by the workflow's own type `W`, which provides each with `Provides<dyn Notifier>`, whose `role(&self)` returns it. A hook requests one as `#[role] notifier: &dyn Notifier`, and without the macros takes it with `got.role::<dyn Notifier>()`, which compiles only where `W: Provides<dyn Notifier>`, so a role the workflow does not provide is not expressible. A role operation that can fail returns `Result<_, itinera::error::Error>`, and the hook propagates it, which aborts with `hook failed`.
- **Panics are never caught.** A panic is a bug: it propagates to whoever called `run`, and the journey stops as if the process had crashed.
- **Each hook has a synchronous trait and an asynchronous one**, behind the `async` feature. A `Synchronous` workflow's hooks are synchronous, and an `Asynchronous` workflow's are asynchronous.

### 6. Workflow descriptors and input adapters

```rust
static ORDERS: LazyLock<WorkflowDescriptor<Orders, Synchronous>> = LazyLock::new(|| {
    WorkflowDescriptor::builder("orders")
        .step(StepDescriptor::new(step_name!("charge"), ChargeFactory)
            .retry_budget(2)
            .no_retry_on_abnormal_termination()
            .policy(StepPolicyDescriptor::new("audit", Audit::default).on_step_success()))
        .step(StepDescriptor::new(step_name!("ship"), ShipFactory))
        .policy(WorkflowPolicyDescriptor::new("notify", Notify::new).on_workflow_success())
        .reporter::<AuditLog>()
        .id_generator(|w: &Orders, data: &DataBag| Ok(format!("order-{}", w.next_number())))
        .build()
        .expect("the orders workflow is well formed")
});
```

- **A workflow descriptor** is the workflow's declaration: its name, its step descriptors, its policy descriptors, its input adapters, its reporters and its ID generator. It is immutable, `Send + Sync`, cheap to clone, shared by every instance, and only `build()` can produce one.
- **Names are newtypes fixed at compile time**: `WorkflowName`, `StepName`, `PolicyName` and `AdapterName`, each over a `&'static str`, so they are `Copy` and every event carries them without copying text. A workflow is declared before any journey, and the macros name its parts from the code itself, so a name never needs to be made at run time. `StepName::new` panics on an empty name, and `step_name!("charge")` calls it in a `const` block, which makes an empty literal a compile error. `JourneyId`, made for each instance at run time, holds an `Arc<str>`. Events, aborts and the result hold these types instead of `String`.
- **Adapter names are unique**: the builder panics when an input adapter is given a name the workflow already uses. The macros refuse two input adapters of one name at compile time (section 11).
- **Each step descriptor states everything**: the retry budget, a `u16` (0 unless set), `retry on abnormal termination` (true unless turned off with `no_retry_on_abnormal_termination()`), and its policies in the order attached. A `u16` budget allows at most 65,536 attempts, so every attempt number fits the `NonZeroU32` events carry, and counting attempts can never overflow while a journey runs.
- **`build()` checks what types cannot**, and returns every violation at once in `Violations`: `duplicate step name`, `hook defined twice`, `input adapter for unknown step` and `step adapted twice`.
- **`listing()`** gives each step's name, its position from 1, its policy names in order, its input adapter's name, the services it requests with the provider serving each, and, for a step with a rescue step, a rescue section listing the rescue step's name, input adapter and services in the same way.
- **Input adapters are hooks of the workflow**, written as its own annotated methods and attached to whole steps, at most one per step:

  ```rust
  #[workflow]
  impl Orders {
      #[input_adapter(steps = ["charge", "refund"])]
      fn pricing(&self, #[step_name] step: StepName, #[key] key: &str,
                 #[from_workflow("price")] price: i64) -> Result<Option<AnyValue>, Error> { /* ... */ }
  }
  ```

  It is called for each input of an adapted step. A value is used; `None` means "not mine", and the input is read from the data bag; `Err` emits `input_adapter_failed` and aborts with `step could not be built`. Its value is untyped and checked against the step's declared type. It may request the step's name, the key, data from the workflow and the journey ID, never roles, a contributor or a reporter. The macro generates the builder call that registers it: `InputAdapterDescriptor::new(name, step, adapt)`, where `adapt` is a function of the workflow's own value and a `Requested<'_, W, InputAdapter>`, the same as a policy's hook receives, of the hook kind `InputAdapter`. `.needing(needs)` declares its requests for data from the workflow, which the engine resolves from the data bag before each call, with the same events and aborts as a hook's, naming the adapter. `.step(other)` attaches it to another step.
- **An input adapter may also read the data bag itself** (proposal 0091). It declares read access with `HookNeeds::data_bag()` and takes it with `got.data_bag()?`, a `DataBagAccess<'_>` valid for the call. `access.read::<T>(key)` gives a `Read<T>`: `Present(value)`, its own copy, `Absent` or `OtherType`. Reading emits nothing and aborts nothing, and the access offers no way to write.
- **Service providers are part of the workflow too** (proposal 0095, stage 10). A step declares a service with `StepNeeds::service(&TOKEN)`, a typed `Service<T>` token, and borrows it for its attempt with `Resolved::service(&TOKEN)`. The workflow declares each provider with `WorkflowBuilder::provider(ServiceProviderDescriptor::new(name, &TOKEN, provide))`, of journey scope, or `::per_attempt(...)`, and may attach it to steps with `.step(name)`. A provider is a hook of the hook kind `Provider<S>`, where the scope `S` decides what it may request, so a journey-scoped provider asking for the attempt does not compile. `build()` reports the service violations, the listing shows which provider serves each service, and the engine keeps journey-scoped instances until the journey ends and drops attempt-scoped ones with their attempt. A rescue step requests services with `RescueNeeds::service`, and is served by the provider attached to its name, `<step>_rescue`, or else by the one attached to no step, never by one attached to the step it rescues (proposal 0098, whose tech spec is [docs/specs/0098-rescue-step-adapters-and-providers.md](specs/0098-rescue-step-adapters-and-providers.md)). A provider returns its instance as a `Provided<T>`, made with `provide!(instance)` or `provide!(instance, data)`, and `service_provided` names the instance's concrete type and carries the data (proposal 0104, whose tech spec is [docs/specs/0104-service-implementation-on-events.md](specs/0104-service-implementation-on-events.md)). The details are in [docs/specs/0095-services-provided-to-steps.md](specs/0095-services-provided-to-steps.md).
- **Fakes in place of service providers, for tests**, behind the `testing` feature. `fake_provider::<T>(name, fake)`, on the instance builder and on the builder `#[workflow]` generates, puts `fake`, a function returning `Result<Provided<T>, Error>`, in place of the function of the provider named `name`, for that instance alone. The provider keeps its scope, the steps it is attached to and its needs, so its requests are still resolved first and `fake` is called wherever the provider would be; an error from it aborts the journey as the provider's would. A provider is named by text, so the test names `T`, such as `dyn PaymentGateway`. A name the workflow has no provider of, or a `T` other than the provider's service type, panics where the fake is given, or at `create()` for the builder `#[workflow]` generates, which takes the fake only with the feature on.

### 7. The workflow instance

The specification requires only that any executor can run any instance, however it was produced. In Rust, that contract is one trait, the only thing an executor relies on. The executor coordinates the flow; it does not own data.

```rust
pub trait WorkflowInstance: Send + Sized + 'static {
    type Workflow: Send + Sync + 'static;
    type Mode: Mode;
    fn descriptor(&self) -> &WorkflowDescriptor<Self::Workflow, Self::Mode>;
    fn workflow(&self) -> &Self::Workflow;
    fn journey_id(&self) -> &JourneyId;
    fn take_reporters(&mut self) -> Vec<<Self::Mode as Mode>::Reporter>;
    fn take_global_metadata(&mut self) -> GlobalMetadata { GlobalMetadata::new() }
    fn data_bag(&self) -> &DataBag;
    fn into_data_bag(self) -> DataBag;
    fn commit(&mut self, key: String, value: AnyValue) -> Committed;
}
```

- **The workflow's own type `W`** holds what its code needs, such as services, and implements its roles. Roles and input adapters see it; no policy's hook ever sees the data bag.
- **itinera provides `Instance<W>`**, created through a builder that the macros also use:

  ```rust
  let instance = ORDERS.instance(Orders::new(services))
      .data("amount", 42_i64)
      .create()?;
  ```

  A workflow declared with the macros has a builder of its own, `Orders::builder()`, with a setter per field of its struct (section 11). `create()` produces the journey ID first, with the descriptor's generator (a closure returning a `Result<String, Error>`, which may read the initial data) or the default UUID version 4, then makes the instance. A failure of the generator is an `InstanceError`, outside any journey. `InstanceBuilder::reporters_global_metadata(key, value)` gives the instance initial global metadata, which the executor sets after the journey ID.
- **Reporters are listed on the workflow by type, or made for one instance.** A listed reporter implements `Reporter + Default`, and every instance makes its own with `Default`; an asynchronous one implements `AsyncReporter + Default`, and only an `Asynchronous` workflow lists one. A reporter made for one journey, such as one that knows the request the journey serves, is added with `InstanceBuilder::reporter(r)`, or `async_reporter(r)` on an asynchronous workflow's instance. They receive events in order: the listed ones first, in order, then those added, in order. Every reporter receives the journey's global metadata with `set_global_metadata(&str, &AnyValue) -> Result<(), Error>`, which does nothing by default: the executor sets the journey ID under `journey_id`, then the instance's initial global metadata, before `journey_started`, and steps, hooks and service providers set more through their reporters while the journey runs. Reporters never see the workflow's own value: what they need, they are given when they are made. A mode says how its workflows hold reporters: a `Synchronous` one as `Box<dyn Reporter>`, an `Asynchronous` one as `BoxedReporter`.
- **The instance holds the data; the engine resolves requests.** The engine reads the data bag through `data_bag()` and resolves every request from it: a step's inputs after its input adapter, and the requests of hooks and input adapters. `commit` writes and says whether a value was replaced, `Committed::Added` or `Committed::Overwritten`, which the engine turns into events.
- **A hand-written instance** implements the trait itself, and points at a descriptor that only `build()` can produce.
- **An instance runs once**: `run` takes it by value, and `into_data_bag` gives up the instance for the data bag the result carries.

### 8. Executors

- **`LocalExecutor`** runs `Synchronous` workflows. **`AsyncLocalExecutor`**, behind the `async` feature, runs `Asynchronous` workflows, calling their synchronous reporters inline.
- **One engine.** The scan, the decisions, the hooks and the events are written once, as asynchronous code. The synchronous executor drives it with `std::task::Waker::noop()`: a `Synchronous` workflow has nothing to wait for, so it runs straight through.
- **No runtime dependency.** The engine never spawns, sleeps, sets timers or cancels. It only awaits the futures of the workflow's own parts, one after another, so it runs on any async runtime.
- **`run` returns the journey's result, or a refusal before the journey starts**:

  ```rust
  impl<F: DispatcherFactory> LocalExecutor<F> {
      pub fn new() -> LocalExecutor<DefaultDispatcherFactory>;
      pub fn with_dispatcher_factory(factory: F) -> Self;
      pub fn run<I: WorkflowInstance<Mode = Synchronous>>(&mut self, instance: I) -> Result<JourneyResult, Refusal>;
  }
  ```

  A refusal comes from custom code called before the journey starts: a workflow policy that cannot be built, a dispatcher factory that fails, or a dispatcher that fails while the journey's reporters are added. Declaration violations are settled by `build()`, modes by types, and the journey ID and reporters by `create()`. `&mut self` makes concurrent journeys on one executor impossible to write.
- **`local_executor_for!` picks the executor of an instance's mode**, since `#[workflow]` takes the mode from the steps and the application may never write it. `local_executor_for!(instance)` makes a `LocalExecutor` or an `AsyncLocalExecutor` with the `DefaultDispatcherFactory`, and `local_executor_for!(instance, dispatcher_factory = factory)` one with `factory`, which must implement `LocalDispatcherFactory<M>` for the instance's mode `M`: `DispatcherFactory` for `Synchronous`, `AsyncDispatcherFactory` for `Asynchronous`. A macro, because the factory is optional; it expands to a hidden function generic over the instance, which reads the mode. The caller still awaits an asynchronous executor's `run`.
- **Everything asynchronous is awaited**: asynchronous steps, hooks, reporters and dispatchers alike.
- **A dropped future abandons the journey.** If the caller drops the future returned by `AsyncLocalExecutor::run`, the journey stops where it is. The documentation says so.
- **The future is `Send`** whenever the instance is: the engine holds no reference to the instance across an await, so an instance need not be `Sync`.
- **Deferred:** cooperative yields between units of the async engine.

### 9. Events, reporters and dispatchers

- **One `Event` type**: a sequence number from 1, a `Timestamp` (a `SystemTime` that displays in ISO 8601 in UTC), the journey ID, the workflow name, and a typed body with one variant per event of the catalogue. `kind()` returns the event's snake_case name. No variant has a field able to hold a value from the data bag. Hook names are `StepHook` or `WorkflowHook`, and an event from or about a hook carries the step and attempt only for a step hook. Who decided follows from each decision, and names only a hook that may have decided it: `journey_failed` by `FailWorkflow` holds a `DecidingHook` (a policy and any step hook), `step_given_up` by `FailWorkflow` one whose hook is a `GiveUpHook` (`on step retry` or `on step abnormal termination`), and `journey_succeeded` by `FinishWorkflow` the policy whose `on step success` returned it. Every other decision, and every retry, is decided by default. Events are the domain's and reporters are its adapters, so events carry no format: neither `Event` nor any type it carries implements `Serialize`. Each reporter writes events as it chooses, from their fields, `kind()`, and names that display as the specification writes them. Only itinera constructs events: `Event` and its variants are `#[non_exhaustive]`, so code outside the crate can read them but not make them. The types events carry live with their concepts: `JourneyId`, `AbortReason`, `FailureCause` and `LastFailure` in `journey`, `StepAttempt` and `Reason` in `step`, the hook names and `Lifecycle` in `policy`. What only events need stays in `event`: who did or decided something, the decisions' causes, and `JourneyFailure` and `JourneyAbort`, which tell how a journey ended with errors as messages.
- **`journey_failed` and `journey_aborted` are events about the journey.** They name the step but carry no attempt number. `journey_failed` carries the cause, the reason when there is one and the error's message when there is one; `journey_aborted` has one variant per abort reason, holding the step when there is one, the details, and the error's message when failing custom code caused the abort. Events never carry an `itinera::error::Error`, only its `Display` text.
- **Data in step and hook events, and reason details,** are values carried in memory, cloned when emitted, and serialized only by reporters, in their own format.
- **Reporters**: `Reporter` with `report(&mut self, &Event) -> Result<(), Error>` and `set_global_metadata(&mut self, &str, &AnyValue) -> Result<(), Error>`, which receives an entry of global metadata and does nothing by default, and `AsyncReporter` behind the `async` feature. One reporter comes ready-made, for trying a workflow out, debugging one and watching a test run: `DebugReporter`, which prints each event's `Debug` on a line of its own to standard output and drops a line standard output cannot take. It is the only reporter the crate provides for applications to run with; any other format is the application's own reporter. Behind the `testing` feature, which an application enables for its tests, `RecordingReporter` keeps every event it receives, shared between its clones, so a test adds a clone to an instance and reads the events once the journey is over; it also keeps the latest value of each entry of global metadata, read with `global_metadata()`, and `path()` lists the flow of the recorded journey, one plain line for each attempt's outcome, each decision on a step and the journey's end, with no timestamp, sequence number or ID; itinera's own tests use it too. A reporter is expected to handle its own trouble (log it, drop the event, retry later) and return `Ok`, and the documentation says so. An `Err` aborts the journey with `reporter failed`, and delivery of that event stops at that reporter: the reporters after it never receive it. `journey_aborted` then goes to every reporter except the one that failed, so the order of reporters matters, and the documentation says that too. The executor wraps each reporter it adds for the journey, so this holds whatever the dispatcher: once a reporter has failed, the wrappers let only `journey_aborted` through, and never to the one that failed. An error while `journey_aborted` itself is delivered is ignored. Only an `Asynchronous` workflow lists an asynchronous reporter.
- **The handles given to steps and hooks** are restricted views of the journey's dispatcher. A `StepReporter` emits only `step_info`, `step_warning` and `step_error`, stamped with the step and attempt; a `HookReporter` emits only `journey_info`, `journey_warning` and `journey_error`, stamped with the policy and hook. Both, and their asynchronous forms, also set entries of global metadata with `set_global_metadata`, which delivers the entry, then emits `global_metadata_set`, and is interrupted like an emit call. Delivery happens before the emit call returns. If a reporter fails during that delivery, the engine records the abort at once, and the call returns `Interrupted`. From then on nothing the step or hook emits is delivered, and when it returns, its outcome, lifecycle and contributions are ignored, even if it ignored `Interrupted` and carried on. In an `Asynchronous` workflow, the handles are asynchronous: an emit call is awaited, and the step or hook waits there until every reporter has the event.
- **Recorders for tests**, behind the `testing` feature, lend the handles that only the executor makes otherwise, so that a test calls a step's `run`, a hook or an input adapter directly, outside any journey, and reads back what it did. `journey::Contributions` lends a `Contributor` and keeps what is contributed, read with `read::<T>(key)`, which returns a `Read<T>`, and `keys()`. `step::StepEventReporter` lends a `StepReporter`, or an `AsyncStepReporter` with `async`, and `policy::HookEventReporter` lends a `HookReporter` or an `AsyncHookReporter`; each keeps one `event::Emitted` per event, with its `event::Level`, message and data but no source, and the global metadata that was set. Made with `failing()`, a recorder interrupts every call and keeps nothing; setting `journey_id` interrupts that call and every later one, as in a journey. `DataBag::default().with(key, value)` builds a data bag and `access()` lends its `DataBagAccess`. Behaviour that belongs to the journey, such as committing only on success, stays with tests that run a journey.
- **Scripted runs for tests**, behind the `testing` feature, check a workflow's flow without its steps' business rules. `instance.scripted()` starts a script that names steps by text, rescue steps by their own names, with `.step(name)` and `.and_step(name)`, and says what their attempts do: `on_attempt(n)`, attempt 1 when not said, `on_every_attempt()` and `on_remaining_attempts()`, then, in order, `will_contribute`, `will_emit`, `will_emit_with` and `will_set_global_metadata`, and an ending: `will_succeed()`, `will_fail(reason)`, `will_skip()`, `will_skip_because(reason)`, `will_terminate_abnormally(message)`, or `will_run_for_real()` straight after the attempt is chosen. Each scripted step is still built for real, its inputs, services and constructor included, and only its `run` is replaced; a step the script does not name runs for real. `run()`, awaited for an asynchronous workflow, runs the journey on a default executor and returns a `ScriptedRun` holding the `JourneyResult` and a record of each step. Before the journey, `run()` panics, listing what the script leaves out: an attempt a scripted step can have, the rescue step of a scripted step that can fail on attempt 1, and, after `ensure_all_scripted()`, a step without a script. `run.step(name)`, which panics on a name the workflow does not have, gives every attempt handed to the construction of its step, scripted or real, with `attempts()` and `attempt(n)`. Each attempt reads what its step was built with through `input::<T>(key)`, which returns a `Read<T>` and, for a rescue step, also reads the contributions it requested from the failed attempt. The ending a script plays is public as `step::Ending`, with `outcome()`, so a test tool that writes real steps can end them the same way. A mistake in writing the script, such as an unknown step, a gap in the attempts, a rescue step that skips, or a contribution, emit or entry of global metadata for a step that declares no contributor or reporter, panics where it is written.
- **Dispatchers**: `Dispatcher` and `AsyncDispatcher`, with `add`, `dispatch` and `set_global_metadata`, all returning `Result<(), Error>`. `LocalExecutor` is given a `DispatcherFactory` and `AsyncLocalExecutor` an `AsyncDispatcherFactory`, or each uses `DefaultDispatcherFactory`, which implements both. A synchronous dispatcher accepts only synchronous reporters; an asynchronous one accepts a `BoxedReporter`, which holds either kind. The executor calls the factory once per journey, before the journey starts, adds the instance's reporters, and drops the dispatcher when the journey ends. `DefaultDispatcher` holds `Box<dyn Reporter>`, or `BoxedReporter` for the asynchronous executor, and nothing else, since a sealed trait bounds its parameter. It calls reporters in the order they were added, and stops at the first that fails, except on `journey_aborted`, whose delivery a failure does not stop; leaving out the reporter that failed is the work of the executor's wrappers. It passes each entry of global metadata to every reporter in the same order, and stops at the first that fails.
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
    PolicyCouldNotBeBuilt { policy: PolicyName, error: Error },
    RequiredDataMissing(MissingData),
    WrongType { key: String, requester: Requester },
    HookFailed(Error),
    ReporterFailed(Error),
    ReservedMetadataKey(MetadataSetter),
}
```

- **The result is a business outcome.** It holds no step statuses, attempt counts or step names: those are observable in the event stream.
- **The status is one enum carrying its payload**, so a failed journey without a failure, a succeeded one with an abort, or an aborted one with data, cannot be written. `JourneyStatus::kind()` returns a plain `StatusKind` (`Succeeded`, `Failed`, `Aborted`) for code that only needs the status.
- **An aborted journey has no data bag.** An abort means something illegal happened: it has no business outcome, and an aborted journey is never resumed. The data bag lives only in `Succeeded` and `Failed`, so reading the data of an aborted journey cannot be written. `JourneyStatus::data()` returns `Option<&DataBag>` for code that handles every status alike.
- **A failure has one variant per cause, and an abort one per abort reason**, each holding exactly what proposal 0083 says it carries: a reason, an error, or both, and the abort's details. `Failure::cause()` and `Abort::reason()` give the plain `FailureCause` and `AbortReason`. They mirror the events' `JourneyFailure`, `JourneyAbort`, `MissingData` and `Requester`, which hold the error's message and name the step, and share `LastFailure` with them, holding the error itself where events hold its message. The result's `MissingData` and `Requester` are the same shapes without the step's name, in `journey` with the result, and so is `MetadataSetter`, who set global metadata under `journey_id`, the counterpart of the event's `Source`. `MissingData` is a key and who requested it, or a step hook's request for the failure's reason or the error, which have no key.
- **The result holds the error itself**, an `itinera::error::Error`, so the caller can inspect it; events carry only its message. As an error cannot be cloned in general, `JourneyResult` is not `Clone`.
- **`AbortReason`** has the reasons Rust can reach, `StepCouldNotBeBuilt`, `RequiredDataMissing`, `WrongType`, `PolicyCouldNotBeBuilt`, `HookFailed`, `ReporterFailed` and `ReservedMetadataKey`, and is `#[non_exhaustive]`. `invalid lifecycle` and `not a value` cannot happen.
- **`Refusal`** names what refused the journey (initial global metadata that sets `journey_id`, a workflow policy, the dispatcher factory, the dispatcher, or a failure while the global metadata is set) and carries its `Error`, when there is one.
- **`Violations`** (from `build()`), **`InstanceError`** (from `create()`) and **`Refusal`** implement `std::error::Error`.

### 11. Macros

The macros are syntax over the builder. Each expands to builder calls and to implementations of public traits of the core, which an application can write by hand, so no API is reachable only through them. They live in `itinera-macros`; `itinera` re-exports each from the module of its concept, behind the `macros` feature, which is on by default, and generated code names `::itinera`. A mistake in a declaration that a macro can see is a compile error pointing at what was written. The one exception is `#[async_trait]`, behind `async` rather than `macros`, which hands its item and arguments to the attribute of async-trait through the hidden re-export `mode::expansion`, so that a trait with `async fn` methods can be a trait object.

| Written | On | Generates |
|---|---|---|
| `#[derive(Step)]` | a step's struct | `FromInputs`, building the step from its fields |
| `#[step]` | the step's `impl` block, holding `run` and, without the derive, `new` | `DescribedStep`, with a hidden factory and step implementing `StepFactory` and `Step`, or their asynchronous traits, and, from `new`, `FromInputs` |
| `#[derive(RescueStep)]` | a rescue step's struct | `FromRescueRequests`, building the rescue step from its fields |
| `#[rescue_step]` | the rescue step's `impl` block, holding `run` | `DescribedRescueStep`, with a hidden factory and rescue step implementing `RescueFactory` and `RescueStep`, or their asynchronous traits |
| `#[step_policy]`, `#[workflow_policy]` | the policy's `impl` block | `DescribedPolicy`, `StepPolicy<W>` or `WorkflowPolicy<W>`, and one hook trait per hook |
| `#[workflow]` and `#[with_step]` | the workflow's struct | `Workflow`, and the instance builder |
| `#[workflow]` | the workflow's `impl` block | its input adapters, its ID generator and its service providers |
| `#[role]` | a role's implementation for the workflow | `Provides<dyn Role>` |
| `#[derive(InitialData)]`, `#[derive(Output)]` | a struct of journey data | `InitialData`, `Output` |
| `#[async_trait]` | a trait whose methods are `async fn`, and each of its implementations | the same trait and implementations, through async-trait, with boxed `Send` futures, so that the trait can be a trait object |

`#[with_step]`, the markers of methods and the attributes of parameters are not imported: the macro that owns them reads and removes them.

- **Steps.** With `#[derive(Step)]`, each field of the struct is an `#[input]`, whose key is the field's name unless written, as in `#[input("coupon")]`, or a `#[default]`, which starts from `Default`. Without the derive, `new` in the block builds the step from its `#[input]` parameters, returning `Self` or `Result<Self, Error>`, whose `Err` is a step that could not be built. `run(self, ...)` takes the handles it marks `#[contributor]` or `#[reporter]`; `async fn run` makes the step asynchronous. `run` and `new` also take services, by shared reference, as in `#[service("payments")] gateway: &dyn PaymentGateway`, keyed by the parameter's name when no key is written; a service both take is declared once and is the same type in both. A step has no name of its own: the workflow names it where it lists it. Every function of the block may write `Self::succeed()`, `Self::fail(reason)`, `Self::skip()` and `Self::skip_because(reason)`, which `DescribedStep` provides; `reason` is anything that converts into a `Reason`, such as its code.
- **Rescue steps.** With `#[derive(RescueStep)]`, each field of the struct is one request, in the order the fields are written: an `#[input]`, a `#[from_rescued_step]`, whose key is the field's name unless written, a `#[reason]` or an `#[error]`. `#[rescue_step]` reads `run` as `#[step]` does, services included, and its block may write `Self::succeed()` and `Self::fail(reason)`, which `DescribedRescueStep` provides; there is no skip. A rescue step has no name of its own, and its attribute takes no arguments.
- **Optional requests.** Whether an input or a request is optional is said by its type: an `Option<T>` is optional, and is `None` when its key is absent. A required datum whose value is itself an `Option` adds `required`, as in `#[input("note", required)]`. The macros read only the tokens written, so an alias of `Option` is not recognised.
- **Policies.** A policy is a plain type whose hooks are methods of one block, each marked with its hook, such as `#[on_step_success]`, taking `&self` and returning the hook's own type (section 5); `async fn` hooks make the policy asynchronous. Each parameter after `&self` is one request: `#[from_step("key")]` and `#[from_workflow("key")]`, whose key is the parameter's name when none is written, `#[step_name]`, `#[attempt]`, `#[reason]`, `#[cause]`, `#[error]`, `#[journey_id]`, `#[role]`, `#[contributor]` and `#[reporter]`. Requests resolve in the order of the parameters. The policy is named after its type unless the attribute names it, as in `#[step_policy(name = "audit")]`, and is built with `Default` unless a function of the block marked `#[factory]` builds it, possibly failing. Hook bodies may write `Self::carry_on()`, `Self::fail_workflow(reason)` and, on step success only, `Self::finish_workflow()`, which `DescribedPolicy` provides.
- **Roles.** `#[role]` on `impl Notifier for Orders` keeps the implementation and provides the role. A policy requesting `#[role] notifier: &dyn Notifier` is implemented only for workflows that provide it. An asynchronous hook may hold its role across an await, and its future must be `Send`, so a role used by asynchronous hooks is `Sync`, as in `trait Notifier: Sync`.
- **Workflows.** `#[workflow(...)]` on the struct takes, each optional, `name = "..."` (the type's name otherwise), `policies = [...]`, `reporters = [...]` and `async_reporters = [...]`. The workflow takes its mode from its steps: the mode of its first step, which every other step shares, so a workflow of asynchronous steps is `Asynchronous`; the step's type is named only in a constant, so that a public workflow may list private steps. Below it, one `#[with_step(Charge, name = "charge", ...)]` per step, in the order the steps run, with `retry_budget = N`, `no_retry_on_abnormal_termination`, `policies = [...]` and `rescue = R`, each optional. On the workflow's `impl` block, `#[workflow]` reads the methods marked `#[input_adapter(steps = ["charge"])]`, named after the method unless `name = "..."` is given, which may request `#[step_name]`, `#[key]`, `#[from_workflow]`, `#[journey_id]` and `#[data_bag]` and the one marked `#[id_generator]`, which takes `&self` and may take the initial data as `&DataBag`. It also reads the service providers: methods marked `#[provider]`, of journey scope, or `#[provider(per_attempt)]`, with `steps = [...]`, `key = "..."` and `name = "..."`, each optional, which take `&self`, return `Result<Provided<T>, Error>`, and may request `#[journey_id]`, `#[from_workflow]`, `#[reporter]` and, of attempt scope, `#[step_name]` and `#[attempt]`; an `async fn` provider belongs to an asynchronous workflow. `Orders::descriptor()` builds the descriptor the first time it is asked for, keeps it in a static, and shares it with every instance. Every function of the block may write `Self::supply(value)` and `Self::from_data_bag()`, which `Workflow` provides.
- **Instances.** `Orders::builder()` has one setter per field of the struct, taking the field's type. A field is required unless it is an `Option` or marked `#[default]`, and `create()` compiles only once every required field is set. `.initial_data(data)` takes any `InitialData`, `.reporters_global_metadata(key, value)` adds an entry of initial global metadata, and `.reporter(r)` and `.async_reporter(r)` add reporters made for this journey. The builder keeps what it is given until `create()`, as boxed calls, so initial data is `'static` and the builder is not `Send`.
- **Journey data.** `#[derive(InitialData)]` makes each field one key, named after the field unless `#[key("...")]` names it; an `Option` field set to `None` leaves its key out, unless it is `#[required]`. `#[derive(Output)]` reads the same shape with `result.output::<Receipt>()`, which fails with an `OutputError` naming the key that is missing or holds another type, or saying that the journey aborted. `DataBag::required` and `DataBag::optional` read one key the same way.
- **Checked at compile time.** Every violation `build()` would report is a compile error, except `wrong service type` on a key whose providers write their return types differently, or other than as `Result<Provided<T>, …>`, so the descriptor of a workflow the macros declare is otherwise always built: a duplicate step name, counting the names of rescue steps, a rescued step with a retry budget, a hook defined twice on a step or on the workflow, an input adapter for a step the workflow does not list, counting the names of rescue steps, and a step adapted twice. So are a workflow without steps, which specification chapter 3 does not require but the agreed API refuses; a step without a name; two input adapters of one name; an empty list of adapted steps; two ID generators; steps of both modes; asynchronous reporters in a synchronous workflow, listed or added to an instance; asynchronous reporters without the `async` feature; a field named like a builder method; a key given by two fields of journey data; a request a hook's kind may not make; a role the workflow does not provide; and every misused attribute. So are a service no provider serves, a service requested as a concrete type rather than a trait object, one function requesting a service twice, a service provided twice to no step or to one step, two providers of one name, a provider for a step the workflow does not list, counting the names of rescue steps, a request a provider's scope may not make, a provider of the other mode, and a service requested as another type than its providers make, through bounds of a hidden function of each step and rescue step on what the workflow's providers of each key make. Only a key whose providers write their return types differently, or other than as `Result<Provided<T>, …>`, is left to `build()`, and the first request for the descriptor panics with `wrong service type` there, as tech spec 0095 explains. An asynchronous policy in a synchronous workflow is a type mismatch on its mode, as is a rescue step whose mode is not its step's. Should a hand-written `DescribedStep`, `DescribedRescueStep`, `StepPolicy` or `WorkflowPolicy` break its contract, the first request for the descriptor panics with the violations.
- **Generics are refused.** A generic step, policy, workflow or struct of journey data is a compile error: Rust has no generic statics for a workflow's descriptor, and the others are described with the builder or by implementing their traits by hand.

## Testing

### 12. The conformance runner

- **A binary**, `itinera-conformance`, run with `cargo run -p itinera-conformance --release`, using cucumber-rs on Tokio (a dependency of the runner only). It reads the cases from `ITINERA_CONFORMANCE_CASES`, selects scenarios with the tag expression in `ITINERA_CONFORMANCE_TAGS`, writes Cucumber JSON to `ITINERA_CONFORMANCE_REPORT`, and fails if any selected scenario fails or uses an undefined sentence.
- **Only the public API.** Each scenario's sentences fill a scenario model, which "When the workflow runs" turns into builder calls: scripted steps implementing `StepFactory`, scripted policy factories recording what their hooks received, a scripted workflow type implementing one recording role trait, scripted reporters, and a recording dispatcher factory whose dispatchers share the test's reporter. Every sentence about events, and about step statuses, reads from that reporter. Its scripted steps end each attempt with `step::Ending`, from the `testing` feature, as scripted runs do. When a scenario uses the default dispatcher, the runner gives no factory: a sentence about one reporter reads that reporter, and a sentence about the whole event stream reads the first of the scenario's reporters that never failed, a failure on `journey_aborted` included; if every one of them failed, the runner reports the scenario as an error in the case. Scripted failures (a hook, role operation, reporter, dispatcher, factory or policy that fails) return an `Error`.
- **The scenario model.** The Given sentences fill a scenario model, plain data that uses itinera only for the hook names, and fail when a sentence does not fit what the scenario declared so far, such as a step the workflow does not have. The When and Then sentences arrive with the stages whose API they need. A sentence used only by scenarios excluded as impossible to express is never defined, since those scenarios never run.
- **Both executors, as the rows of an Examples table.** As it reads the cases, the runner copies every scenario once for each executor that runs it, as if the scenario had an Examples table with one row per executor: one for `LocalExecutor`, tagged `@executor-local`, and one for `AsyncLocalExecutor`, tagged `@executor-async`. A scenario tagged `@capability-sync` gets only the first, and one tagged `@capability-async` only the second. Under `AsyncLocalExecutor`, the runner declares the scenario's workflow `Asynchronous`, with asynchronous scripted steps and hooks. Each copy is a scenario of its own, which must pass, and keeps the scenario's name. Like a row of a real Examples table, each has a line of its own, since the Cucumber JSON report tells scenarios apart by name and line: the first copy has the scenario's line, and the next is moved past every scenario sharing its name, such as the other rows of a Scenario Outline.
- **Test support through itinera's own.** The scenarios tagged `@capability-testing` reach the recording reporter, fake providers, scripted runs and recorders only through the `testing` feature, as an application's tests do. A scripted run uses the local executor of the workflow's mode, with no dispatcher factory from the runner, so the runner's own recorder is the instance's first reporter. A mistake in a test is a panic, which the runner catches and reads back into the names the cases give mistakes. Tech spec 0108 says where each piece lives.
- **The runner knows nothing of excluded scenarios.** It writes only the Cucumber JSON, holding the scenarios that ran.
- **Exclusions are declared in `conformance.json`.** `impossible` maps each excluded tag to its scenarios, each with its feature file, its name, and its proof: the test that proves it and the file holding that test. Every scenario carrying the tag at the pinned cases must have an entry, so a tag is added, with all its entries, in the pull request that lands the proofs for all its scenarios. Until then its scenarios do not run anyway, since their proposals are not listed yet.
- **Proofs are Rust tests, run by `cargo test`.** They live in `crates/itinera/tests/proofs.rs`, with their fixtures beside it in `crates/itinera/tests/proofs/<tag>/`, because they test the public API, not the runner. Each excluded scenario has its own `#[test]`, which runs `trybuild` on one fixture holding the forbidden code, compared with the compiler error it must produce. Its twin, which keeps the rule, is a module of the `proofs` test target, so it compiles with the proofs, and the test runs the twin's `main` unless that is empty. The fixtures share a few support types. The proofs are:
  - `invalid-lifecycle`: `on step failure` returning `FinishWorkflow`; the twin returns `FailWorkflow`;
  - `role-not-provided`: a policy whose hook requests a role, attached to a workflow that does not implement `Provides` for it; the twin's workflow provides the role;
  - `mode-not-accepted`: an asynchronous step run by `LocalExecutor`, and an asynchronous service provider given to a synchronous workflow; the twin gives it to an asynchronous workflow;
  - `non-value`: a closure as initial data, as a contribution, as an adapter's value, as event data, a service provider's included, and as a reason's details;
  - `late-handle`: a contributor and step reporter kept by the step's factory beyond the attempt, or moved into a thread that may outlive it; the twin uses them from a scoped thread, which ends within the attempt. Also an input adapter's access to the data bag kept by the workflow beyond the call, whether the adapter takes the call's own lifetime or claims a `'static` one, which `InputAdapterDescriptor::new` refuses; the twin reads through it during the call;
  - `bag-write`: an input adapter writing through its access to the data bag; the twin reads through it;
  - `rescue-skip`: a rescue step returning a skip; the twin succeeds;
  - `journey-scope-request`: a service provider of journey scope requesting the attempt; the twin is of attempt scope;
  - `no-instance`: a service provider returning nothing; the twin returns a `Provided`.
- **The `run-conformance` action** checks the exclusions and writes the report's second file. It leaves the `impossible` tags out of the tag expression, fails if a tagged scenario has no entry or an entry names no scenario at the pinned cases, and writes the exclusions file next to the Cucumber JSON. Together the two files are the conformance report a release carries, written to `conformance-report/cucumber.json` and `conformance-report/exclusions.json` and uploaded as an artifact. While `conformance.json` lists no proposal, the action still checks the manifest but does not run the runner, so its job is part of CI from stage 0.
- **What runs.** A scenario runs once every proposal it is tagged with is listed in `conformance.json`. Listing a proposal therefore runs those of its scenarios whose other proposals are already listed; the rest join in, by themselves, when their last proposal is listed.
- **The tag expression** the action passes in `ITINERA_CONFORMANCE_TAGS` uses `and`, `or`, `not` and parentheses, for example `(@proposal-0002 or @proposal-0008) and not @non-value`. The runner parses it with cucumber-rs's tag expressions, which stage 2 tests.

### 13. Rust's own tests

- **Unit tests** in `itinera-core`, next to the code: the decision rules, typed reads from the bag, the order of events, the handling of `Interrupted`.
- **Macro equivalence**: each macro form is paired with the same workflow written with the builder. Both must give the same listing, and the same event stream for the same inputs, apart from the journey ID and timestamps.
- **Compile-fail tests** in `crates/itinera/tests/ui/`, each with a compiling twin: running an instance twice, calling `run` concurrently, attaching a policy that defines no hook, and each refusal of the macros listed in section 11, such as a hook request its kind may not make. A few of them check a refusal made only without the `async` feature, such as an `async fn` step: they run with `cargo test -p itinera --test ui` without that feature, which no CI job does, since the test job enables every feature. Each twin is a module of the `ui` test target, compiled once with it, and the test runs its `main` unless that is empty; only the programs that must fail go through `trybuild`, which checks them without building them.
- **Macro features**: the conformance runner also runs `@macros` features of its own, in `crates/itinera-conformance/features/macros`, which drive workflows declared only with the macros through the cases' own sentences.
- **Property-based tests** with `proptest`, over generated workflows with random outcomes, budgets, lifecycles and failing hooks or reporters: the first and last events, increasing sequence numbers, one outcome fact per attempt, one decision after each failed attempt, at most the budget plus one attempts, no bag values in engine events, and identical streams from both executors.
- **Doc tests**: every public item has an example that compiles and runs.
- **No `unsafe`**: `#![forbid(unsafe_code)]` in every crate.
- Coverage may be reported for information; it never blocks a merge.

### 14. Continuous integration

Each job runs one command, as the organisation's rules require. All are required to merge, except the weekly beta run.

| Check | Command |
|---|---|
| format | `cargo fmt --all --check` |
| lints | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| tests, on Linux | `cargo test --workspace --all-features` |
| tests, on Windows and macOS | `cargo test --workspace --all-features --exclude itinera` |
| sync-only build | `cargo check --workspace --no-default-features` |
| minimum Rust version | `cargo +1.85 check --workspace --exclude itinera-conformance` |
| documentation | `cargo doc --workspace --no-deps`, with `RUSTDOCFLAGS=-D warnings` |
| dependencies | `cargo deny check`, with the `cargo-deny` binary that `taiki-e/install-action` installs |
| conformance, from stage 0 | `itinera-dev/actions/run-conformance@v1`, with `command: cargo run -p itinera-conformance --release`, after checking out and restoring the Rust cache |
| pull request rules | `pr-has-issue` and `no-cross-repo-closing` |
| weekly, not blocking | the tests on Rust beta |

The tests of the `itinera` crate, which holds only re-exports, run on Linux only: most of them compare compiler messages, which read the same on every system.

The toolchain comes from `rust-toolchain.toml`; caching uses `Swatinem/rust-cache`; third-party actions are pinned to a commit.

## Order of work

### 15. Stages

| Stage | Content | Proposals completed |
|---|---|---|
| 0. Bootstrap | the workspace and crates, `rust-toolchain.toml`, lints, CI, `deny.toml`, the README, an agents' manual, `conformance.json` pinning the latest cases candidate, with the capabilities, no proposals, and `"impossible": {}` | none |
| 1. Events | `Error`, `Value` and `AnyValue` (the data bag waits for stage 3), `Event` and the types it carries, reporters, dispatchers and their factories, `DefaultDispatcher` | none |
| 2. Runner skeleton | cucumber-rs, the environment variables, the recording dispatcher factory, the scenario model, the Cucumber JSON report | none |
| 3. Minimal executor | the engine and both executors, `WorkflowDescriptor` with one synchronous step that can only succeed, the data bag holding the initial data, `WorkflowInstance` and `Instance<W>`, journey IDs, reporters built by `create()`, `JourneyResult` and `Refusal` | none |
| 4. Declarations and admission | step and policy descriptors, input adapter declarations, the listing, `Violations`; until stages 5 and 7, a step is a closure that can only succeed, a policy descriptor names the hooks it defines, which keep their default behaviour, and an input adapter supplies no value; a step's retry budget and `retry on abnormal termination` come with stage 6, which uses them | 0062 |
| 5. Steps | modes declared from the start, needs and tokens, building per attempt, input adapters that supply values (stage 7 adds their requests), outcomes, contributions, typed reads from the data bag, read-only received data, values, handles and `Interrupted`; until stage 6, every attempt is the last, as if each step had a retry budget of 0, so a failed attempt ends the journey with the default decisions `step_given_up` and `journey_failed` | 0056, 0057, 0064 (their scenarios are all proven impossible) |
| 6. The scan and its decisions | statuses, retries, `retry on abnormal termination`, aborts and the result; the decision logic with its hook points in place, tested through internal test hooks; it replaces stage 5's rule that every attempt is the last | 0012, 0032, 0042, 0061, 0063, 0065 |
| 7. Policies, hooks and roles | policy descriptors and factories, every hook kind and its requests, lifecycles, roles, workflow hooks, input adapters as hooks | 0002, 0008, 0009, 0010, 0011, 0024, 0027, 0040, 0041, 0049, 0054, 0055, 0058, 0060, 0081, 0083, 0085 |
| 8. Macros | reporters given the journey ID, `#[step]` and `derive(Step)`, `#[step_policy]` and `#[workflow_policy]`, `derive(InitialData)` and `derive(Output)`, `#[workflow]`, `#[with_step]` and `#[role]` with the instance builder, their equivalence and compile-fail tests, the `@macros` features, the crate overview and the examples | none |
| 9. Release | the release workflow, `release-gate`, `0.1.0-rc.1` | none |
| 10. Services | `Service<T>` tokens, service providers of journey and attempt scope attached to steps or not, their requests and events, the service violations and the listing's services; the runner's sentences; `#[service]` and `#[provider]` in the macros; the `journey-scope-request` and `no-instance` proofs, and those of providers for `mode-not-accepted` and `non-value` | 0095 |
| 11. Rescue steps | rescue steps and their traits, outcome, needs and factories, the `rescued step retried` violation, the rescue in the listing, the decisions `step_rescuing` and `step_rescued`, the engine's rescue, the runner's sentences, the `rescue-skip` proof, then the macros | 0003 |

Proposal 0091, which lets an input adapter read the data bag, joined tier 1 after stage 7. It was completed by the change that moved the cases to `v0.1.0-rc.8`, outside the stages.

- **The decision logic is pure**: what the step's own rule decides after a failed attempt is a function of how the attempt ended, its number, the retry budget and `retry on abnormal termination`, which touches nothing. The engine calls the step hooks around it, in the order of chapter 6, and a hook's lifecycle overrides the default. Until stage 7, every hook point answered nothing, and the engine's tests scripted their answers; since stage 7, they call the policies' hooks. The code around it builds, runs, emits and commits.
- **Step statuses are not public.** The engine's scan keeps them to itself, as its position in the steps and attempts, and the conformance runner derives each step's status and attempt count from the event stream, as the specification allows.
- **Proofs land with their feature**, together with their tag's entries under `impossible` in `conformance.json`. A proposal is listed in `conformance.json` only when each of its scenarios that runs once it is listed passes, or is proven.
- **A later cases candidate** is adopted by a pull request that moves `cases` in `conformance.json` and fixes whatever its changed cases need.

### Pull requests

- Each stage is one stack (`gh stack`), with one layer per coherent piece. Every layer passes all of `main`'s checks.
- Every layer says `Refs #N` for the implementation issues it contributes to. The layer that completes a proposal says `Closes #N` and adds the proposal's number to `conformance.json`.
- Work that belongs to no proposal (stage 0, the macros, the release) refers to #7, or to an issue of its own such as "Macros as syntax over the builder".
- Public API of a proposal not yet listed in `conformance.json` stays behind the `unstable` feature, and so does a listed proposal's API while it holds a stand-in that a later stage replaces, or names API of a proposal not yet listed. Stage 7 lists every tier 1 proposal accepted by then, 0091 was listed once accepted, and 0003 and 0095 by the last layers of stages 11 and 10, so every module is public, and `unstable` gates only API that the specification has not accepted yet, of which there is none now.

## Issues and tech specs

### 16. Implementation issues

- **One issue per tier 1 proposal**, titled "Implement spec#NNNN (short name)", labelled `implements-proposal` and `tier: 1`. Its body links the proposal and its cases, summarises the decisions of this plan that concern it, lists its excluded scenarios and their proofs, and says it closes only when its cases pass.
- **[#2](https://github.com/itinera-dev/itinera-rs/issues/2)** is the implementation issue of 0002. Its comments are kept as history; the plan supersedes them.
- **Spec updates.** Once a proposal is listed in `conformance.json`, a later change to its cases gets a `spec update` issue, as PROCESS.md describes.
- **Tech specs**, `docs/specs/NNNN-short-name.md`, hold what is specific to one proposal: its API, how each of its rules is enforced (by types, by `build()`, or while running), its excluded scenarios and proofs, its own tests, and its definition of done. A tech spec is created in the first pull request that touches its proposal and completed in the one that closes it.

## Code

### 17. Style, structure and comments

- **Formatting**: plain `rustfmt`, without configuration.
- **Names** use the specification's vocabulary exactly: `JourneyId`, `StepDescriptor`, `Contributor`, `Outcome`, `retry_budget`, `FailWorkflow`. Event kinds keep their snake_case names.
- **No panics while a journey runs**: every failure there is a value, never a panic used as a throw. Before a journey, a mistake in a declaration that types cannot rule out may panic, with a message saying what was wrong. Clippy denies `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `todo` and `unimplemented` outside tests; a function allowed to panic at declaration says so with `#[expect(clippy::panic, reason = "…")]` and documents it.
- **A small public surface**: `pub(crate)` by default, checked by `unreachable_pub`; public types that may grow are `#[non_exhaustive]`.
- **Modules of `itinera-core` follow concepts**, named with the specification's vocabulary: `error`, `value`, `journey` (the journey ID, the journey's data and how it ends, the result included), `step`, `policy`, `event` (events, and what only events carry), `report` (reporters and dispatchers), `mode` (execution modes), `workflow` (descriptors, input adapters and violations), `instance`, `executor`, and the private `engine`. A type lives with its concept, not where it is first used, and modules may use one another. A module with submodules re-exports its public face, so public paths have one module level, such as `itinera::journey::JourneyId`; the crate root exports modules, not types.
- **Comments are as few as possible.** They explain a non-obvious reason only when the code cannot. Code and comments never refer to specification sections, issues, pull requests or other documents: git keeps that history, and the tech specs map rules to code. There are no `TODO` comments.
- **Public documentation** describes each item's behaviour in its own words, with an example. `missing_docs` is an error.
- **Test names** state the rule as a sentence, for example `a_failed_attempts_contributions_are_never_committed`.
- **Table tests**: a rule checked on several inputs is one `rstest` test with a named case per input; when what varies is a type, each case holds a generic function instantiated for one type, passed as a function pointer. A loop over every value of a closed set stays a loop.
- **British spelling** in documentation, as in the specification.

## Tooling

### 18. Toolchain, dependencies, features and documentation

- **`rust-toolchain.toml`** pins Rust 1.98.1 with `rustfmt` and `clippy`. Moving to a newer version is a pull request of its own, which also refreshes the stored compiler messages.
- **The minimum supported Rust version is 1.85**, declared as `rust-version`. Before 1.0, raising it is allowed in a minor release, and the changelog says so. It is a promise to users of the published crates, so the conformance runner, which is not published, follows cucumber-rs instead: it declares 1.88, and the minimum-version check leaves it out.
- **Dependencies**, each with default features off:

  | Crate | Used by | For |
  |---|---|---|
  | `serde`, `erased-serde` | core | the value bound, letting reporters serialize type-erased values |
  | `derive_more` (`debug`, `display`, `from`, `into`, `as_ref`, `into_iterator`) | core | `Debug` on types whose type parameters need not be `Debug`, or whose fields are skipped, summarised or forwarded, and `Display`, `From`, `Into`, `AsRef` and `IntoIterator` on newtypes, names and collections, which the types use throughout |
  | `uuid`, `getrandom` (`std`) | core; `uuid` also in the conformance runner | the default journey ID, a UUID v4 made from random bytes, so that a failing random source is an error rather than a panic; the runner checks with `uuid` that a journey ID is a UUID v4 |
  | `thiserror` | core, conformance runner | `Display` and `std::error::Error` on error enums, such as `InstanceError` |
  | `async-trait` | core | the engine's own trait objects whose methods are asynchronous, in both modes; and, through the `async_trait` attribute that `mode` re-exports from the macros with `async`, the services and roles of asynchronous workflows, which steps and hooks use as trait objects |
  | `syn`, `quote`, `proc-macro2` | macros | the macros |
  | `cucumber` (`macros`, `output-json`), `tokio` (`rt`), `futures`, `serde_json` | conformance runner | running the cases |
  | `trybuild`, `proptest`, `serde_json` | tests | compile-fail, property-based and equivalence tests |
  | `futures` (`executor`) | tests | running asynchronous journeys in tests |
  | `rstest` | tests | one test per rule, with a named case per input |

  No async runtime and no `futures` crate in the core. A new dependency of the core needs a stated reason in its pull request.
- **`deny.toml`** allows the licences MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0 and Zlib, and BlueOak-1.0.0 for the three `synthez` crates only, which cucumber's macros use in the runner. It denies security advisories, accepts crates only from crates.io, and warns on duplicate versions.
- **Features of `itinera`** (forwarded to `itinera-core`), all additive:

  | Feature | Default | Adds |
  |---|---|---|
  | `macros` | on | the macros |
  | `async` | off | the asynchronous executor, steps, hooks, reporters and dispatchers, the `Asynchronous` mode, and the `async_trait` attribute in `mode`, which brings in the macros crate |
  | `unstable` | off | API of proposals the specification has not accepted yet; empty for now |

  The engine is asynchronous internally whatever the features; without `async`, nothing asynchronous is public.
- **Documentation**: rustdoc on every public item, published by docs.rs; a crate-level overview in `itinera` with a first complete workflow, linking once to the specification repository; runnable examples in `crates/itinera/examples/` (a synchronous workflow, the same on the asynchronous executor, and one built without macros); a README stating the crates, the tier and capabilities claimed, the rules made impossible, and how to run the conformance runner.

## Release

### 19. Releasing the crates

- **Crates**: `itinera-core`, `itinera-macros` and `itinera`, with one workspace version, published together by `cargo publish --workspace`.
- **Versions are independent of the specification's.** The first release is `0.1.0-rc.1`. The README and the crate documentation say what each release implements: "specification 0.1.0, tier 1, capabilities `sync`, `async` and `testing`, cases `v0.1.0-rc.N`".
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

### 20. The rest

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
