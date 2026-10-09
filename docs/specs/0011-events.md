# 0011: Events

Tech spec for [proposal 0011](https://github.com/itinera-dev/spec/blob/main/proposals/0011-events.md), implemented in [#13](https://github.com/itinera-dev/itinera-rs/issues/13). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 adds the events, reporters and dispatchers; the engine that emits the events arrives in stages 3 to 7.

## API

- **`Event`** carries a sequence number from 1, a `Timestamp`, which displays in ISO 8601 in UTC, the `JourneyId`, the workflow name, and an `EventBody` with one variant per event of the catalogue. `kind()` gives the snake_case name.
- Only itinera constructs events: `Event` and every `EventBody` variant are `#[non_exhaustive]`, readable but not constructible outside the crate.
- An event about a step carries a `StepAttempt`; one from or about a hook carries a `HookSource`, either a step hook with its policy and the triggering `StepAttempt`, or a workflow hook with its policy and no step.
- `optional_input_absent` carries a `RequestSource`: the step for its input, the hook, or the input adapter with the step it was resolving for.
- Events carry no format: no event type implements `Serialize`, and each reporter writes events as it chooses. Hooks, causes, lifecycles and abort reasons display as the specification writes them.
- **`StepReporter`** and, behind the `async` feature, **`AsyncStepReporter`**, in `itinera::step`, emit a step's own events: `info`, `warning` and `error`, each with a `_with` form adding data. Each call delivers the event before it returns, or is awaited until then, and returns `Result<(), itinera::error::Interrupted>`.
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
| A reporter that fails aborts the journey | the executors, in stage 3; reporters can only fail by returning `Err` |
| A step emits only `step_info`, `step_warning` and `step_error`, stamped with its step and attempt | types: a step reporter has only those methods, and stamps each event with the attempt it was made for |
| Delivery is blocking | a step reporter's call returns, or its future completes, only once the event was delivered |
| A reporter that fails on a step's event ends the step, and the abort stands whatever it does next | the call returns `Interrupted` and the engine records the abort at once; it delivers nothing more for the step, and ignores its outcome and contributions |
| Event data is a value, captured when emitted | types: the `_with` methods take a `T: Value` by value |

## Tests

- Unit tests in `itinera-core/src/policy.rs` and `journey.rs`: hooks and lifecycles, and causes and abort reasons, display as the specification writes them.
- Unit tests in `itinera-core/src/event.rs`: every kind of event has its own kind; names display as the specification writes them; a timestamp displays in UTC in ISO 8601; a timestamp just before 1970 borrows from the previous second; a year outside 0 to 9999 is written as an expanded year.
- Unit tests in `itinera-core/src/report.rs`: with no reporter, dispatching succeeds and goes nowhere.
- Unit tests in `itinera-core/src/engine.rs`: a step's own events come before its outcome, stamped with its attempt, in both modes; a reporter failing on one aborts the journey, and nothing the step does afterwards is delivered or committed.

## Excluded scenarios

The scenario "Data that is not a value aborts the journey, and the event is not delivered", tagged `@non-value`, is proven impossible to express by `a_step_event_cannot_carry_a_non_value` in `crates/itinera/tests/proofs.rs`.

## Done when

Every scenario tagged `@proposal-0011` passes, and 11 is listed in `conformance.json`. Planned for stage 7.
