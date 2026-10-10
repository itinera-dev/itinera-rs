# 0060: Input adapters are attached to steps, and leave unknown inputs to the data bag

Tech spec for [proposal 0060](https://github.com/itinera-dev/spec/blob/main/proposals/0060-input-adapters-attached-to-steps.md), implemented in [#27](https://github.com/itinera-dev/itinera-rs/issues/27). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7. Stage 4 declared input adapters attached to steps, with their names, their violations and their place in the listing. Stage 5 made them supply values, leaving to the data bag the inputs they do not supply. Stage 7 made them hooks of the workflow, of the kind `InputAdapter`, with their own requests.

## API

- **`itinera::workflow::InputAdapterDescriptor<W>`** declares an input adapter. `InputAdapterDescriptor::new(name, step, adapt)` gives its name, an `AdapterName`, the first step it is attached to, and its function. `.step(other)` attaches it to another step, and `.needing(needs)` declares what it requests, a `HookNeeds<InputAdapter>`. `WorkflowBuilder::input_adapter` adds it to the workflow.
- **The function** takes the workflow's own value and a `Requested<'_, W, InputAdapter>`, and returns `Result<Option<AnyValue>, Error>`: a value, `None` for "not mine", or the error it failed with. Its signature fits a method of the workflow's own type, such as `Orders::pricing`. It is a plain function in both execution modes, since it emits nothing and has nothing to wait for.
- **What an adapter may request**: the step's name, with `Requested::step_name()`, the key being resolved, with `Requested::key()`, the journey ID, with `Requested::journey_id()`, and data from the workflow, which it declares with `HookNeeds::from_workflow` or `optional_from_workflow` and takes with `Requested::from_workflow` or `optional_from_workflow`. `HookNeeds<InputAdapter>` offers nothing else, and its `Requested` offers no role, contributor or reporter.
- **Behind the `unstable` feature, `Requested::data_bag()`** gives an adapter a shared reference to the data bag, to read: for example, to derive an amount from a price and a quantity. The specification does not allow it yet; [spec#91](https://github.com/itinera-dev/spec/issues/91) proposes it, and the method stays behind `unstable` until it is accepted. Reading emits no event and aborts nothing.
- **The listing** gives each step's input adapter as `ListedStep::adapter`, an `Option<AdapterName>`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| An input adapter is declared on the workflow and attached to one or more of its steps | types: `InputAdapterDescriptor::new` takes its first step, and only `WorkflowBuilder::input_adapter` adds it to a workflow |
| A step has at most one input adapter | `build()` refuses `step adapted twice` |
| An adapter attached to a step that does not exist is refused | `build()` refuses `input adapter for unknown step` |
| Every adapter has a name, unique among the workflow's adapters | types: its constructor takes the name; `WorkflowBuilder::input_adapter` panics on a name the workflow already uses, since that is a mistake in a declaration, before any journey |
| For each input of an adapted step, in order, the adapter is called | the engine, while it resolves the step's inputs |
| A value is used, and `input_adapter_supplied` names the adapter; a value of another type than the step declared aborts with `wrong type`, naming the adapter | the engine checks the value's exact type against the step's token |
| Nothing means "not mine": the input is resolved from the data bag as if the step had no adapter, and the abort, if any, does not name the adapter | the engine falls back to the data bag, with the step as the requester |
| An adapter that fails emits `input_adapter_failed`, naming it, and aborts with `step could not be built` | the engine |
| An adapter receives only what it declares: the step's name, the key, data from the workflow and the journey ID | types: `HookNeeds<InputAdapter>` declares only data from the workflow, and `Requested<'_, W, InputAdapter>` offers only those four |
| An adapter requests no role, and receives no contributor and no means to emit events | types: `InputAdapter` is not a `PolicyHookKind`, so `role`, `contributor` and `reporter` do not exist for it |
| Its data from the workflow is read from the data bag, never through an adapter: a required value not found aborts with `required data missing`, a wrong type with `wrong type`, and an optional value not found is absent and emits `optional_input_absent`, each naming the step, the adapter and the key | the engine resolves the adapter's requests before each call, with `Requester::Adapter` and `RequestSource::Adapter` |
| The adapter's own `optional_input_absent` events come before `input_adapter_supplied` or `input_adapter_failed`, or before the events of the default resolution | the engine resolves the adapter's requests before calling it |
| The listing shows each step's adapter by name | `listing()`, through `ListedStep::adapter` |
| Adapters serve steps only, never hooks | the engine asks an adapter only while it resolves a step's inputs |

## Tests

- Unit tests in `itinera-core/src/engine.rs`: `an_input_adapter_supplies_an_input_before_the_data_bag_is_read`; `an_input_adapter_that_fails_aborts_the_journey_as_its_step_could_not_be_built`; `a_value_of_another_type_from_an_input_adapter_aborts_the_journey_with_wrong_type`; `an_input_adapter_that_returned_nothing_is_not_named_in_the_abort`; `an_input_adapter_receives_the_data_from_the_workflow_it_requests`; `an_input_adapters_absent_optional_data_is_reported_before_what_it_supplied`; `an_input_adapters_required_data_from_the_workflow_aborts_the_journey_before_it_runs`; `an_input_adapter_is_told_the_step_it_supplies_for_each_of_its_inputs`; with the `unstable` feature, `an_input_adapter_reads_the_data_bag_without_events`.
- Unit tests in `itinera-core/src/workflow/violation.rs`: the cases `two_adapters_on_one_step` and `an_adapter_on_a_step_the_workflow_does_not_have` of `a_workflow_put_together_wrongly_is_refused_with_its_violation`, and `two_input_adapters_with_one_name_cannot_be_declared`.
- Unit tests in `itinera-core/src/workflow.rs`: `a_workflow_is_listed_in_order_with_its_policies_and_adapters_without_running_anything`.
- A proof in `itinera/tests/proofs.rs`, `an_input_adapter_cannot_supply_a_non_value`, belongs to the tech spec of [0056](0056-data-bag-values.md).

## Done when

Every scenario tagged `@proposal-0060` passes, and 60 is listed in `conformance.json`. Done in stage 7.
