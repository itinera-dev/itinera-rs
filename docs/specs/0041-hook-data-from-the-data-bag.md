# 0041: Hooks read data from the workflow without input adapters

Tech spec for [proposal 0041](https://github.com/itinera-dev/spec/blob/main/proposals/0041-hook-data-from-the-data-bag.md), implemented in [#19](https://github.com/itinera-dev/itinera-rs/issues/19). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7, which added hooks and their requests.

## API

- **`HookNeeds::from_workflow`** and **`optional_from_workflow`** declare data from the workflow with the tokens steps use, `Input<T>` and `OptionalInput<T>`, for every kind of hook. The hook takes it with `Requested::from_workflow` or `optional_from_workflow`, already of type `T`.
- **Aborts and events name the hook.** A request that cannot be resolved aborts the journey with `required data missing`, holding a `MissingData::Key`, or with `wrong type`, each with the key and a `Requester`, which is `Requester::StepHook` or `Requester::WorkflowHook`, with the policy and the hook; in events the requester also names the step a step hook acts on. `optional_input_absent` for a hook's request carries `RequestSource::Hook`, with the policy and the hook.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Data from the workflow, for every hook, is read from the data bag under its key, by type, never through an input adapter | the engine reads a hook's data from the workflow from the data bag only, and asks a step's input adapter only while it resolves that step's inputs |
| Data from the step is exactly what the step contributed | the engine resolves it from the attempt's contributions |
| Everything a hook requests is resolved before it runs; if resolution fails, the hook does not run and has no `hook_called` | the engine resolves the requests in the order declared, then calls the hook; the first request that fails ends the journey before the call |
| A required value not found aborts with `required data missing`; a value of the wrong type aborts with `wrong type`, whether required or optional | the engine, which checks each value's exact type against the token |
| `journey_aborted` names the policy, the hook and the key | types: `JourneyAbort::RequiredDataMissing` holds a `MissingData::Key`, and `JourneyAbort::WrongType` the key, each with a `Requester` naming the policy and hook |
| `optional_input_absent` for a hook's request carries the policy and hook as well as the key | types: it carries `RequestSource::Hook` |
| A hook's input events come before its `journey_*` events and its `hook_called` | the engine emits them while it resolves the requests, before the hook runs |

## Tests

- Unit tests in `itinera-core/src/engine/hooks.rs`: `a_step_hook_receives_what_it_requests_from_the_step_and_the_workflow_before_it_runs`, which also checks that each absent optional request emits `optional_input_absent` naming the hook, before `hook_called`; `a_required_request_without_a_value_aborts_the_journey_before_the_hook_runs`; `a_value_of_another_type_for_a_hook_aborts_the_journey_naming_the_hook`.
- Unit tests in `itinera-core/src/event.rs`: `an_abort_while_data_was_resolved_names_the_step_it_was_for_but_no_error` and `an_abort_by_a_workflow_hooks_request_names_no_step`.
- The conformance scenario of 0010 tagged `@proposal-0041`, "A step hook's data from the workflow comes from the data bag, not from the step's input adapter", runs under both executors.

## Done when

Every scenario tagged `@proposal-0041` passes, and 41 is listed in `conformance.json`. Done in stage 7.
