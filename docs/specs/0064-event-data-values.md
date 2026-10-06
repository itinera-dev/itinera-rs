# 0064: Event data and reason details are values, serialized only by reporters

Tech spec for [proposal 0064](https://github.com/itinera-dev/spec/blob/main/proposals/0064-event-data-values.md), implemented in [#31](https://github.com/itinera-dev/itinera-rs/issues/31). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 adds the event data and reason details; the handles that emit events arrive in stage 5.

## API

- The data of `step_info`, `step_warning`, `step_error`, `journey_info`, `journey_warning` and `journey_error` is an `Option<AnyValue>`.
- `Reason::with_details` takes any `T: Value`, captured when the reason is made.
- Itinera never serializes either. `Event` implements `Serialize` so that a reporter can, in whatever format it chooses; the data serializes as the value itself.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Event data and reason details are values | types: only a `T: Value` becomes an `AnyValue` |
| The event carries its own copy, captured when emitted | ownership: emitting takes the value; `AnyValue` is cloned, never shared |

## Excluded scenarios

The scenario of this proposal is tagged `@non-value`, and is proven impossible to express in stage 5.

## Tests

- Unit tests in `itinera-core/src/event.rs`: data emitted by a step is serialized as the value itself.

## Done when

Every scenario tagged `@proposal-0064` is proven impossible to express, and 64 is listed in `conformance.json`. Planned for stage 5.
