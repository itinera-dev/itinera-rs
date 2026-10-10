# 0085: An aborted journey's result carries no data bag, and an abort is final

Tech spec for [proposal 0085](https://github.com/itinera-dev/spec/blob/main/proposals/0085-aborted-result-has-no-data.md), implemented in [#48](https://github.com/itinera-dev/itinera-rs/issues/48). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7. Stage 3 shaped the result so that an aborted journey has no data bag; stage 6 produced every abort of steps, and stage 7 those of policies and hooks, after which no hook runs.

## API

- **`JourneyStatus::Aborted(Abort)`** holds the abort and nothing else. Only `Succeeded` and `Failed` hold a `DataBag`.
- **`JourneyStatus::data()`** returns `None` for an aborted journey, for code that handles every status alike.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| An aborted journey's result carries no data bag | types: `JourneyStatus::Aborted` holds no `DataBag`, so reading it cannot be written |
| An aborted journey is never resumed | types: a `WorkflowInstance` runs once, since `run` takes it by value; nothing can be made from a result |
| After an abort, no hook runs, workflow hooks included | the engine ends the journey with `journey_aborted` at once, without calling `on workflow success` or `on workflow failure` |

## Tests

- Unit tests in `itinera-core/src/journey.rs`: `only_a_journey_that_succeeded_or_failed_has_a_data_bag`.
- Unit tests in `itinera-core/src/engine/hooks.rs`: `no_workflow_hook_runs_when_the_journey_is_aborted`.

## Done when

Every scenario tagged `@proposal-0085` passes, and 85 is listed in `conformance.json`. Done in stage 7.
