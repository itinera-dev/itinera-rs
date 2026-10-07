# 0011: Events

Tech spec for [proposal 0011](https://github.com/itinera-dev/spec/blob/main/proposals/0011-events.md), implemented in [#13](https://github.com/itinera-dev/itinera-rs/issues/13). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 adds the events; the engine that emits them arrives in stages 3 to 7.

## API

- **`Event`** carries a sequence number from 1, a `SystemTime` timestamp, the `JourneyId`, the workflow name, and an `EventBody` with one variant per event of the catalogue. `kind()` gives the snake_case name.
- Only itinera constructs events: `Event` and every `EventBody` variant are `#[non_exhaustive]`, readable but not constructible outside the crate.
- An event about a step carries a `StepAttempt`; one from or about a hook carries a `HookSource`, either a step hook with its policy and the triggering `StepAttempt`, or a workflow hook with its policy and no step.
- `optional_input_absent` carries a `RequestSource`: the step for its input, the hook, or the input adapter with the step it was resolving for.
- `Event` implements `Serialize` as one flat map holding `kind`, `sequence`, `timestamp` in ISO 8601 in UTC, `journey_id`, `workflow` and the body's fields.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Every event carries the fixed fields | types: the fields of `Event` |
| An event about a step carries the step and attempt; one about a hook, the policy and hook, and the step and attempt only for a step hook | types: the fields of each `EventBody` variant, and the variants of `HookSource` and `RequestSource` |
| Only the engine emits events | types: events cannot be constructed outside the crate |
| Exactly the engine catalogue, and the six events of steps and hooks | types: one `EventBody` variant per event |

## Tests

- Unit tests in `itinera-core/src/event.rs`: every kind of event serializes with its own kind, an event serializes its fixed fields with a UTC ISO 8601 timestamp, a workflow hook event carries no step, and an optional request names its requester and the step it was for.

## Done when

Every scenario tagged `@proposal-0011` passes, and 11 is listed in `conformance.json`. Planned for stage 7.
