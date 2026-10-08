# 0085: An aborted journey's result carries no data bag, and an abort is final

Tech spec for [proposal 0085](https://github.com/itinera-dev/spec/blob/main/proposals/0085-aborted-result-has-no-data.md), implemented in [#48](https://github.com/itinera-dev/itinera-rs/issues/48). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 3 shapes the result so that an aborted journey has no data bag.

## API

- **`JourneyStatus::Aborted(Abort)`** holds the abort and nothing else. Only `Succeeded` and `Failed` hold a `DataBag`.
- **`JourneyStatus::data()`** returns `None` for an aborted journey, for code that handles every status alike.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| An aborted journey's result carries no data bag | types: `JourneyStatus::Aborted` holds no `DataBag`, so reading it cannot be written |
| An aborted journey is never resumed | types: a `WorkflowInstance` runs once, since `run` takes it by value; nothing can be made from a result |

## Tests

- Unit tests in `itinera-core/src/journey.rs`: only a journey that succeeded or failed has a data bag.

## Done when

Every scenario tagged `@proposal-0085` passes, and 85 is listed in `conformance.json`. Planned for stage 7.
