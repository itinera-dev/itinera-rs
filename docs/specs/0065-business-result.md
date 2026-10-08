# 0065: The journey result is a business outcome

Tech spec for [proposal 0065](https://github.com/itinera-dev/spec/blob/main/proposals/0065-business-result.md), implemented in [#32](https://github.com/itinera-dev/itinera-rs/issues/32). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 3 adds the result's types, and the executors that return them; stage 6 produces every status.

## API

- **`itinera::journey::JourneyResult`** holds the `journey_id` and the `status`. It is `#[non_exhaustive]`, so only itinera makes one, and it is not `Clone`, since it may hold an `Error`.
- **`JourneyStatus`** has one variant per final status, each carrying its payload: `Succeeded { data }`, `Failed { failure, data }` and `Aborted(Abort)`. `kind()` gives the plain `StatusKind`, and `data()` the data bag of a journey that succeeded or failed.
- **`Failure`** has one variant per cause, and **`Abort`** one per abort reason; the tech spec of 0083 says what each holds.
- **`DataBag`** is the journey's output. It gives the value under a key with `get`, its keys in order with `keys`, and iterates over its keys and values.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| The result carries the journey ID, the final status with what explains it, and the data bag | types: `JourneyResult` and `JourneyStatus` |
| The result carries no step statuses, attempt counts or step names | types: none of the result's types holds one |

## Tests

- Unit tests in `itinera-core/src/journey.rs`: a data bag lists its keys in order, gives the value under a key and iterates over its keys and values; a status names its kind; a failure names its cause; an abort names its reason.

## Done when

Every scenario tagged `@proposal-0065` passes, and 65 is listed in `conformance.json`. Planned for stage 6.
