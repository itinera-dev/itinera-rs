# 0060: Input adapters are attached to steps, and leave unknown inputs to the data bag

Tech spec for [proposal 0060](https://github.com/itinera-dev/spec/blob/main/proposals/0060-input-adapters-attached-to-steps.md), implemented in [#27](https://github.com/itinera-dev/itinera-rs/issues/27). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7. Stage 4 declared input adapters attached to steps, with their names, their violations and their place in the listing. Stage 5 made them supply values, leaving to the data bag the inputs they do not supply. Stage 7 made them hooks of the workflow, of the kind `InputAdapter`, with their own requests. The proof that an input adapter cannot be asynchronous came with the cases' `v0.1.0-rc.19`.

## API

- **`itinera::workflow::InputAdapterDescriptor<W>`** declares an input adapter. `InputAdapterDescriptor::new(name, step, adapt)` gives its name, an `AdapterName`, the first step it is attached to, and its function. `.step(other)` attaches it to another step, and `.needing(needs)` declares what it requests, a `HookNeeds<InputAdapter>`. `WorkflowBuilder::input_adapter` adds it to the workflow.
- **The function** takes the workflow's own value and a `Requested<'_, W, InputAdapter>`, and returns `Result<Supplied, Error>`: `Supplied::Value` with a value, `Supplied::FromDataBag` for "not mine", or the error it failed with. Its signature fits a method of the workflow's own type, such as `Orders::pricing`. It is a plain function in both execution modes, since it emits nothing and has nothing to wait for. A workflow that implements `Workflow`, as every workflow declared with the macros does, may write the value as `Self::supply(value)` and "not mine" as `Self::from_data_bag()`; `Supplied::value(value)` makes the first without it, and `supplied.downcast_ref::<T>()` reads the value back, as an adapter's test does.
- **What an adapter may request**: the step's name, with `Requested::step_name()`, the key being resolved, with `Requested::key()`, the journey ID, with `Requested::journey_id()`, and data from the workflow, which it declares with `HookNeeds::from_workflow` and takes with `Requested::from_workflow`, with an `Input` or an `OptionalInput`. `HookNeeds<InputAdapter>` offers nothing else, and its `Requested` offers no role, contributor or reporter.
- **Read access to the data bag**, declared with `HookNeeds::data_bag()` and taken with `Requested::data_bag()`, comes from proposal 0091, which amends this one; its tech spec is [0091](0091-adapters-read-the-data-bag.md).
- **Attaching an adapter to a rescue step**, by its name, comes from proposal 0098, which amends this one; its tech spec is [0098](0098-rescue-step-adapters-and-providers.md).
- **`duplicate adapter name`**, and a step named twice in one adapter's list, come from proposal 0116, which amends this one; its tech spec is [0116](0116-duplicate-adapter-names.md).
- **The listing** gives each step's input adapter as `ListedStep::adapter`, an `Option<AdapterName>`.
- **`itinera::event::Supplier`** names what failed while a step was built: `Supplier::Adapter`, with the adapter and the key it was resolving, or, since 0095, `Supplier::Provider`. `JourneyAbort::StepCouldNotBeBuilt` holds it as `supplier`, which is `None` when the step's constructor failed or no provider serves one of its services.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| An input adapter is declared on the workflow and attached to one or more of its steps | types: `InputAdapterDescriptor::new` takes its first step, and only `WorkflowBuilder::input_adapter` adds it to a workflow |
| A step has at most one input adapter | `build()` refuses `step adapted twice` |
| An adapter attached to a step that does not exist is refused | `build()` refuses `input adapter for unknown step`; since 0098, the name of a step's rescue step counts as a step's |
| Every adapter has a name, unique among the workflow's adapters | types: its constructor takes the name; since 0116, `build()` refuses two adapters with one name, `duplicate adapter name` |
| For each input of an adapted step, in order, the adapter is called | the engine, while it resolves the step's inputs |
| A value is used, and `input_adapter_supplied` names the adapter; a value of another type than the step declared aborts with `wrong type`, naming the adapter | the engine checks the value's exact type against the step's token |
| Nothing means "not mine": the input is resolved from the data bag as if the step had no adapter, and the abort, if any, does not name the adapter | the engine falls back to the data bag, with the step as the requester |
| An adapter that fails emits `input_adapter_failed`, naming it, and aborts with `step could not be built`, naming the adapter and the key | the engine, with `Supplier::Adapter` in `JourneyAbort::StepCouldNotBeBuilt` |
| An adapter receives only what it declares: the step's name, the key, data from the workflow, the journey ID and, since 0091, read access to the data bag | types: `HookNeeds<InputAdapter>` declares only data from the workflow and read access to the data bag, and `Requested<'_, W, InputAdapter>` offers only those five |
| An adapter requests no role, and receives no contributor and no means to emit events | types: `InputAdapter` is not a `PolicyHookKind`, so `role`, `contributor` and `reporter` do not exist for it |
| Its data from the workflow is read from the data bag, never through an adapter: a required value not found aborts with `required data missing`, a wrong type with `wrong type`, and an optional value not found is absent and emits `optional_input_absent`, each naming the step, the adapter and the key | the engine resolves the adapter's requests before each call, with `Requester::Adapter` and `RequestSource::Adapter` |
| The adapter's own `optional_input_absent` events come before `input_adapter_supplied` or `input_adapter_failed`, or before the events of the default resolution | the engine resolves the adapter's requests before calling it |
| The listing shows each step's adapter by name | `listing()`, through `ListedStep::adapter` |
| Adapters serve steps only, never hooks | the engine asks an adapter only while it resolves a step's inputs |
| An adapter in an execution mode the executor does not accept is refused with `mode not accepted` | types: `InputAdapterDescriptor::new` takes only a plain function, so an asynchronous adapter cannot be written, and every adapter runs under both executors; `#[workflow]` refuses an `async fn` marked `#[input_adapter]` |

## Tests

- Unit tests in `itinera-core/src/engine.rs`: `an_input_adapter_supplies_an_input_before_the_data_bag_is_read`; `an_input_adapter_that_fails_aborts_the_journey_as_its_step_could_not_be_built`; `a_value_of_another_type_from_an_input_adapter_aborts_the_journey_with_wrong_type`; `an_input_adapter_that_returned_nothing_is_not_named_in_the_abort`; `an_input_adapter_receives_the_data_from_the_workflow_it_requests`; `an_input_adapters_absent_optional_data_is_reported_before_what_it_supplied`; `an_input_adapters_required_data_from_the_workflow_aborts_the_journey_before_it_runs`; `an_input_adapter_is_told_the_step_it_supplies_for_each_of_its_inputs`. The tests of its read access to the data bag belong to the tech spec of [0091](0091-adapters-read-the-data-bag.md).
- Unit tests in `itinera-core/src/workflow/violation/admission/adapters.rs`: the cases `two_adapters_on_one_step` and `an_adapter_on_a_step_the_workflow_does_not_have` of `a_workflow_whose_input_adapters_are_declared_wrongly_is_refused_with_its_violation`. Its case of two adapters with one name belongs to the tech spec of [0116](0116-duplicate-adapter-names.md).
- Unit tests in `itinera-core/src/workflow/listing.rs`: `a_workflow_is_listed_in_order_without_running_anything`.
- A proof in `itinera/tests/proofs.rs`, `an_input_adapter_cannot_supply_a_non_value`, belongs to the tech spec of [0056](0056-data-bag-values.md).
- A proof in `itinera/tests/proofs.rs`, `an_input_adapter_cannot_be_asynchronous`: an `async fn` given to `InputAdapterDescriptor::new` fails to compile, and its twin, a synchronous adapter, compiles and runs. It covers the scenario tagged `@mode-not-accepted` below.
- A compile-fail test in `itinera/tests/ui.rs`, `an_input_adapter_is_synchronous`: `#[workflow]` refuses an `async fn` marked `#[input_adapter]`, and its twin, the workflow the macro tests share, compiles and runs.

## Excluded scenarios

The scenario "A synchronous-only executor refuses an asynchronous input adapter before the journey starts", of `cases/tier-1/0012-local-executor/`, is tagged `@proposal-0060` and `@mode-not-accepted`, besides `@proposal-0012`, which its feature carries, and proven impossible to express by `an_input_adapter_cannot_be_asynchronous` in `crates/itinera/tests/proofs.rs`. Since itinera-rs makes an input adapter a plain function in both execution modes, as the API section says, an asynchronous one cannot be written, and no executor meets one.

## Done when

Every scenario tagged `@proposal-0060` passes or is proven impossible to express, and 60 is listed in `conformance.json`. Done in stage 7.
