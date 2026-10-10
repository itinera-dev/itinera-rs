# 0011: Events

Tech spec for [proposal 0011](https://github.com/itinera-dev/spec/blob/main/proposals/0011-events.md), implemented in [#13](https://github.com/itinera-dev/itinera-rs/issues/13). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7. Stage 1 added the events, reporters and dispatchers. Stages 3 to 6 made the engine emit the events of steps and decisions, and stage 7 those of hooks, with the reporters hooks emit through.

## API

- **`Event`** carries a sequence number from 1, a `Timestamp`, which displays in ISO 8601 in UTC, the `JourneyId`, the workflow name, and an `EventBody` with one variant per event of the catalogue. `kind()` gives the snake_case name.
- Only itinera constructs events: `Event` and every `EventBody` variant are `#[non_exhaustive]`, readable but not constructible outside the crate.
- An event about a step carries a `StepAttempt`; one from or about a hook carries a `HookSource`, either a step hook with its policy and the triggering `StepAttempt`, or a workflow hook with its policy and no step.
- `optional_input_absent` carries a `RequestSource`: the step for its input, the hook, or the input adapter with the step it was resolving for.
- Events carry no format: no event type implements `Serialize`, and each reporter writes events as it chooses. Hooks, causes, lifecycles and abort reasons display as the specification writes them.
- **`StepReporter`** and, behind the `async` feature, **`AsyncStepReporter`**, in `itinera::step`, emit a step's own events: `info`, `warning` and `error`, each with a `_with` form adding data. Each call delivers the event before it returns, or is awaited until then, and returns `Result<(), itinera::error::Interrupted>`.
- **`HookReporter`** and, behind the `async` feature, **`AsyncHookReporter`**, in `itinera::policy`, emit a hook's own events with the same methods, `info`, `warning` and `error`, each with a `_with` form, which emit `journey_info`, `journey_warning` and `journey_error`, stamped with the policy and hook, and with the step and attempt for a step hook. A hook declares one with `HookNeeds::reporter` and takes it with `Requested::reporter`.
- **`Reporter`** has `report(&mut self, &Event) -> Result<(), Error>`; **`AsyncReporter`**, behind the `async` feature, returns `impl Future + Send`, so it can be written as `async fn`. Dispatchers and their factories are in the tech spec of [0063](0063-dispatcher-factory.md).

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Every event carries the fixed fields | types: the fields of `Event` |
| The timestamp is in UTC, in ISO 8601 | types: `Timestamp` displays only in that form |
| An event about a step carries the step and attempt; one about a hook, the policy and hook, and the step and attempt only for a step hook | types: the fields of each `EventBody` variant, and the variants of `HookSource` and `RequestSource` |
| Only the engine emits events | types: events cannot be constructed outside the crate |
| Exactly the engine catalogue, and the six events of steps and hooks | types: one `EventBody` variant per event |
| With no reporter, events are still produced and go nowhere | `DefaultDispatcher` with no reporter succeeds |
| A reporter that fails aborts the journey | the executors; reporters can only fail by returning `Err` |
| A step emits only `step_info`, `step_warning` and `step_error`, stamped with its step and attempt | types: a step reporter has only those methods, and stamps each event with the attempt it was made for |
| A hook emits only `journey_info`, `journey_warning` and `journey_error`, stamped with its policy and hook, and with the step and attempt for a step hook | types: a hook reporter has only those methods, and stamps each event with the hook's `HookSource` |
| `hook_called` records the lifecycle a hook returned, and contributions record whether a step or a hook made them | types: `HookCalled` carries an `Option<Lifecycle>`, and `ContributionCommitted` a `Source` |
| Delivery is blocking | a step's or hook's reporter call returns, or its future completes, only once the event was delivered |
| A reporter that fails on a step's or hook's event ends it, and the abort stands whatever it does next | the call returns `Interrupted` and the engine records the abort at once; it delivers nothing more for the step or hook, and ignores its outcome or lifecycle and its contributions |
| Event data is a value, captured when emitted | types: the `_with` methods take a `T: Value` by value |

## Tests

- Unit tests in `itinera-core/src/policy.rs` and `journey.rs`: `hooks_lifecycles_and_causes_display_as_the_specification_writes_them` and `statuses_causes_and_abort_reasons_display_as_the_specification_writes_them`.
- Unit tests in `itinera-core/src/event.rs`: `every_kind_of_event_has_its_own_kind`, `names_display_as_the_specification_writes_them`, `a_timestamp_displays_in_utc_in_iso_8601`, `a_timestamp_just_before_1970_borrows_from_the_previous_second` and `a_year_outside_0_to_9999_is_written_as_an_expanded_year`.
- Unit tests in `itinera-core/src/report.rs`: `with_no_reporter_dispatching_succeeds_and_goes_nowhere`.
- Unit tests in `itinera-core/src/engine.rs`: `events_are_numbered_from_one_and_carry_the_journey_the_workflow_and_the_time`; `a_step_emits_its_own_events_stamped_with_its_attempt_before_its_outcome`; `a_reporter_failing_on_a_step_event_aborts_the_journey_whatever_the_step_does_next`; with the `async` feature, `an_asynchronous_step_awaits_its_own_events` and `a_reporter_failing_on_an_asynchronous_steps_event_aborts_the_journey`.
- Unit tests in `itinera-core/src/engine/hooks.rs`: `a_hooks_contributions_are_committed_once_it_returns_with_the_hook_as_their_source` and `a_reporter_failing_on_a_hooks_event_aborts_the_journey_whatever_the_hook_does_next`.
- Unit tests in `itinera-core/src/executor.rs`: `a_journey_emits_its_events_in_order_to_every_reporter_and_succeeds_with_its_data` and `a_reporter_that_fails_aborts_the_journey_and_only_journey_aborted_reaches_the_others`.

## Excluded scenarios

The scenario "Data that is not a value aborts the journey, and the event is not delivered", tagged `@non-value`, is proven impossible to express by `a_step_event_cannot_carry_a_non_value` in `crates/itinera/tests/proofs.rs`.

## Done when

Every scenario tagged `@proposal-0011` passes or is proven impossible to express, and 11 is listed in `conformance.json`. Done in stage 7.
