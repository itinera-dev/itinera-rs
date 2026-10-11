# 0054: How custom code fails

Tech spec for [proposal 0054](https://github.com/itinera-dev/spec/blob/main/proposals/0054-how-custom-code-fails.md), implemented in [#22](https://github.com/itinera-dev/itinera-rs/issues/22). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7. Stage 1 added the error type, and stages 3 to 7 the executors, steps, hooks and roles that return it. The proofs of the rules made impossible to express landed with their features: those of values and late handles in stage 5, of modes in stage 6, and of lifecycles and roles in stage 7.

## API

- **`itinera::error::Error`** is the error every piece of custom code returns: steps, hooks, role operations, policy factories, reporters, dispatchers and dispatcher factories.
  - It converts from any `std::error::Error + Send + Sync + 'static`, so `?` works on the errors of any library.
  - Because of that conversion it does not implement `std::error::Error`. It offers `Display`, `Debug`, `source()` and `Error::msg`, and converts into `Box<dyn std::error::Error + Send + Sync>`.
- **Abort reasons** are `AbortReason::HookFailed` and `AbortReason::ReporterFailed`, which display as `hook failed` and `reporter failed`.
- **A panic is not caught.** The documentation of `LocalExecutor::run` says that a panic reaches the caller, and the journey stops where it was.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| All custom code fails the same way, by returning an error | types: every trait and function given for custom code returns `Result<_, Error>` |
| What follows depends on what failed: an abnormal termination for a running step, `step could not be built` for a constructor or input adapter, an abort for a hook, role operation, reporter or dispatcher, a refusal before the journey | the engine and the executors, as the tech spec of [0049](0049-custom-code-that-throws.md) lists |
| Unrecoverable failures are outside the model: a panic is never turned into an outcome, and reaches whoever called `run` | itinera never catches a panic |
| `hook failed` and `reporter failed` cover a returned error | the engine turns a hook's `Err` into `Abort::HookFailed`, and a reporter's into `Abort::ReporterFailed` |
| Rules a language may make impossible to express are proven impossible, each excluded scenario by its own test | `crates/itinera/tests/proofs.rs`, and the entries under `impossible` in `conformance.json` |

## Rules made impossible to express

Rust makes all six rules of point 4, the sixth added by proposal 0091, impossible to express, and lists their tags under `impossible` in `conformance.json`. Each excluded scenario is proven by a test in `crates/itinera/tests/proofs.rs`, which compiles the code breaking the rule, expecting the compiler's error, and a twin keeping it, which compiles and runs:

| Tag | Scenario | Proof |
|---|---|---|
| `@invalid-lifecycle` | A lifecycle a hook may not return aborts the journey | `a_failure_hook_cannot_finish_the_workflow` |
| `@role-not-provided` | A policy needing a role the workflow does not provide is refused at admission | `a_policy_cannot_be_attached_to_a_workflow_that_does_not_provide_its_role` |
| `@mode-not-accepted` | A synchronous-only executor refuses an asynchronous step before the journey starts | `a_synchronous_executor_cannot_run_an_asynchronous_step` |
| `@non-value` | A step that contributes a non-value aborts the journey | `a_step_cannot_contribute_a_non_value` |
| `@non-value` | Initial data holding a non-value is refused before any journey | `initial_data_cannot_hold_a_non_value` |
| `@non-value` | An input adapter that supplies a non-value aborts with "step could not be built" | `an_input_adapter_cannot_supply_a_non_value` |
| `@non-value` | A failure whose details are not a value aborts the journey | `a_reasons_details_cannot_be_a_non_value` |
| `@non-value` | Data that is not a value aborts the journey, and the event is not delivered | `a_step_event_cannot_carry_a_non_value` |
| `@late-handle` | A step's contributor and reporter used after its attempt change nothing | `a_steps_handles_cannot_outlive_its_attempt` |
| `@late-handle` | An adapter's access kept from an earlier call shows nothing later | `an_adapters_access_to_the_data_bag_cannot_outlive_its_call` |
| `@bag-write` | An adapter that writes through its access leaves the data bag unchanged | `an_adapter_cannot_write_through_its_access_to_the_data_bag` |

The tech specs of the proposals whose scenarios they are say how each proof works: [0010](0010-hooks-lifecycles-and-roles.md), [0012](0012-local-executor.md), [0056](0056-data-bag-values.md), [0064](0064-event-data-values.md), [0011](0011-events.md), [0057](0057-handles-valid-during-their-attempt.md) and [0091](0091-adapters-read-the-data-bag.md).

## Tests

- Unit tests in `itinera-core/src/error.rs`: `an_error_shows_the_message_of_the_error_it_was_made_from` and `an_error_converts_back_into_a_boxed_standard_error`.
- Unit tests in `itinera-core/src/engine/hooks.rs`: `a_hook_that_fails_aborts_the_journey_with_its_error`.
- Unit tests in `itinera-core/src/executor.rs`: `a_reporter_that_fails_aborts_the_journey_and_only_journey_aborted_reaches_the_others`.
- Unit tests in `itinera-core/src/journey.rs`: `statuses_causes_and_abort_reasons_display_as_the_specification_writes_them`.
- The eleven proofs above, in `itinera/tests/proofs.rs`.

## Done when

Every scenario tagged `@proposal-0054` passes, every scenario carrying one of the six tags is proven impossible to express, and 54 is listed in `conformance.json`. Done in stage 7.
