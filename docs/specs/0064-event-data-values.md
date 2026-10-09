# 0064: Event data and reason details are values, serialized only by reporters

Tech spec for [proposal 0064](https://github.com/itinera-dev/spec/blob/main/proposals/0064-event-data-values.md), implemented in [#31](https://github.com/itinera-dev/itinera-rs/issues/31). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 5. Stage 1 added the event data and reason details, and stage 5 the step reporters that emit events with data, and the proofs.

## API

- The data of `step_info`, `step_warning`, `step_error`, `journey_info`, `journey_warning` and `journey_error` is an `Option<AnyValue>`.
- `Reason::with_details` takes any `T: Value`, captured when the reason is made.
- A step reporter's `info_with`, `warning_with` and `error_with` take any `T: Value`, captured when the event is emitted.
- Itinera never serializes either. Events carry no format; a reporter that writes them serializes the data with `AnyValue`'s `Serialize`, which writes the value itself, in whatever format it chooses.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Event data and reason details are values | types: only a `T: Value` becomes an `AnyValue` |
| The event carries its own copy, captured when emitted | ownership: emitting takes the value; `AnyValue` is cloned, never shared |

## Excluded scenarios

The scenario of this proposal, "A failure whose details are not a value aborts the journey", is tagged `@non-value`, and proven impossible to express by `a_reasons_details_cannot_be_a_non_value` in `crates/itinera/tests/proofs.rs`. Event data that is not a value, checked in the cases of 0011, is proven impossible by `a_step_event_cannot_carry_a_non_value`.

## Tests

- Unit tests in `itinera-core/src/value.rs`: an erased value serializes as the value itself.
- Unit tests in `itinera-core/src/engine.rs`: a step's event carries the data it was emitted with.

## Done when

Every scenario tagged `@proposal-0064` is proven impossible to express, and 64 is listed in `conformance.json`. Done in stage 5.
