# 0002: Foundations

Tech spec for [proposal 0002](https://github.com/itinera-dev/spec/blob/main/proposals/0002-foundations.md), implemented in [#2](https://github.com/itinera-dev/itinera-rs/issues/2). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals. Proposals 0027, 0058 and 0060 amend it, and their tech specs cover what they change: received data is read-only, policies are built per journey or per attempt, and input adapters are attached to steps.

Status: done in stage 7. Stage 4 added the declaration: step and policy descriptors, input adapters, admission with `hook defined twice`, and the listing. Stage 5 built each step from the inputs it declares, resolved from its input adapter or the data bag. Stage 7 added the policies' hooks, with the data they request from the step.

## API

- **A step sees only what it declares.** A `StepFactory`, or an `AsyncStepFactory` behind the `async` feature, declares in `StepNeeds` its inputs, each required as an `Input<T>` or optional as an `OptionalInput<T>`, and whether it takes a contributor and a reporter. It builds the step from `Resolved`, which holds only those. A step descriptor is a `StepDescriptor<W, M>`, but neither its factory nor its step ever receives the workflow's own value, the data bag, the executor or another step.
- **A workflow is declared before any journey.** `WorkflowDescriptor::builder`, or `async_builder`, returns a `WorkflowBuilder`, whose `step`, `policy` and `input_adapter` add the steps in order, the workflow policies and the input adapters, and whose `build()` makes the descriptor. `WorkflowDescriptor::listing()` lists it without running anything.
- **Policies** are described by `itinera::policy::StepPolicyDescriptor`, attached to a step with `StepDescriptor::policy`, and `itinera::policy::WorkflowPolicyDescriptor`, attached to the workflow with `WorkflowBuilder::policy`. A step policy descriptor offers only step hooks, such as `.on_step_success()`, and a workflow policy descriptor only workflow hooks, such as `.on_workflow_success()`. Each hook is a trait of its own, such as `OnStepSuccess<W>`, which the policy's type implements; the tech spec of [0010](0010-hooks-lifecycles-and-roles.md) describes them.
- **Data from the step.** A step hook declares it with `HookNeeds::from_step` or `optional_from_step`, using the same tokens as steps, and takes it with `Requested::from_step` or `optional_from_step`, already of its type. A step policy names no step, and may take the name of the step it acts on with `Requested::step_name()`.
- **Input adapters** are `itinera::workflow::InputAdapterDescriptor<W>`, declared on the workflow; the tech spec of [0060](0060-input-adapters-attached-to-steps.md) describes them as amended.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A step can access only its declared inputs, its contributor and its reporter | types: its factory builds it from `Resolved`, which holds only what `StepNeeds` declared; taking a token or handle the step did not declare is an error from `build`, so the step could not be built |
| A step has no access to the workflow, the data bag, the executor or other steps | types: neither `StepFactory`, `Resolved` nor `Step` names the workflow's type or the data bag |
| Steps, their order, policies and adapters are fixed before the journey, and listable without running anything | types: only `build()` makes a descriptor, which has no method that changes it; `listing()` reads only the declaration |
| A policy defines one or more hooks, all of one kind | types: a policy descriptor offers only the hooks of its kind, and only a `Hooked` one, which has named a hook, can be attached |
| Each hook a policy defines is identified as the hook it implements | types: one trait per hook, and the descriptor names each hook it defines |
| Each step hook is defined at most once per step, and each workflow hook once per workflow, reported at admission with every other violation | `build()`, with `hook defined twice`: `Violation::StepHookDefinedTwice` or `Violation::WorkflowHookDefinedTwice` |
| A hook that no attached policy defines takes its default behaviour | the engine calls a hook only when an attached policy defines it, and otherwise keeps the default |
| Policies are evaluated in the order they are declared | descriptors keep their policies in the order attached, and the engine builds them and looks for each hook in that order |
| An input with no adapter is read from the data bag; a missing required input aborts, a missing optional one is absent; a value of the wrong type aborts, whether required or optional | the engine, while it resolves the step's inputs before building it |
| A failing adapter aborts the journey | the engine emits `input_adapter_failed`, then aborts with `step could not be built` |
| Only step hooks receive data from the step, exactly what it contributed | types: `from_step` exists only for the kinds of step hooks, `StepHookKind`; the engine resolves it from the attempt's contributions, committed or not |
| A step hook's missing required data from the step aborts; a missing optional one is absent; a wrong type aborts, whether required or optional | the engine, while it resolves the hook's requests before the hook runs |

## Tests

- Unit tests in `itinera-core/src/workflow/violation.rs`: `a_workflow_put_together_wrongly_is_refused_with_its_violation`, with a case for two step policies defining one hook, one step policy attached twice and two workflow policies defining one hook; `every_violation_is_reported_together`; `a_workflow_put_together_rightly_is_built`.
- Unit tests in `itinera-core/src/policy.rs`: `a_step_hook_named_twice_is_defined_once_with_the_needs_named_last` and `a_workflow_hook_named_twice_is_defined_once_with_the_needs_named_last`.
- Unit tests in `itinera-core/src/workflow.rs`: `a_workflow_is_listed_in_order_with_its_policies_and_adapters_without_running_anything`.
- Unit tests in `itinera-core/src/step/needs.rs`: `an_input_the_step_does_not_declare_cannot_be_read`, `an_input_read_as_another_type_than_declared_cannot_be_read` and `a_handle_the_step_does_not_declare_cannot_be_taken`.
- Unit tests in `itinera-core/src/engine.rs`: `a_step_is_built_with_the_inputs_it_declares_read_from_the_data_bag`, `an_optional_input_without_a_value_is_absent_and_reported`, `a_required_input_without_a_value_aborts_the_journey_before_the_step_is_built`, `a_value_of_another_type_aborts_the_journey_with_wrong_type`, `an_input_adapter_supplies_an_input_before_the_data_bag_is_read` and `an_input_adapter_that_fails_aborts_the_journey_as_its_step_could_not_be_built`.
- Unit tests in `itinera-core/src/engine/hooks.rs`: `a_step_hook_receives_what_it_requests_from_the_step_and_the_workflow_before_it_runs`, `a_required_request_without_a_value_aborts_the_journey_before_the_hook_runs` and `a_value_of_another_type_for_a_hook_aborts_the_journey_naming_the_hook`.
- A compile-fail test in `itinera/tests/ui.rs`, `a_policy_attached_defines_at_least_one_hook`: attaching a policy descriptor that names no hook does not compile, and its twin, which names `on step success`, compiles and runs.

## Done when

Every scenario tagged `@proposal-0002` passes, and 2 is listed in `conformance.json`. Done in stage 7.
