# 0040: Events record facts and decisions

Tech spec for [proposal 0040](https://github.com/itinera-dev/spec/blob/main/proposals/0040-events-facts-and-decisions.md), implemented in [#18](https://github.com/itinera-dev/itinera-rs/issues/18). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 adds the events; the engine that emits them in the right order arrives in stages 3 to 7.

## API

- The decisions are the `EventBody` variants `StepRetrying`, `StepGivenUp`, `JourneySucceeded` and `JourneyFailed`. Each carries a `DecidedBy`: `Default`, or the `HookRef` (policy and hook) whose lifecycle decided it.
- Causes are enums: `RetryCause` (`retriable failure`, `abnormal termination`), `GiveUpCause` (`failure`, `abnormal termination`, `retries exhausted`, `FailWorkflow` with its reason), and `Failure` for `journey_failed`.
- `hook_called` carries the `Lifecycle` returned, or `None`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Engine events carry keys, never data values | types: no engine event has a field of type `AnyValue`, apart from the details of a reason |
| A decision says who decided it | types: every decision variant has a `decided_by` field |

## Tests

- Unit tests in `itinera-core/src/event.rs`: a decision names who decided it.

## Done when

Every scenario tagged `@proposal-0040` passes, and 40 is listed in `conformance.json`. Planned for stage 7.
