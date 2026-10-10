# 0040: Events record facts and decisions

Tech spec for [proposal 0040](https://github.com/itinera-dev/spec/blob/main/proposals/0040-events-facts-and-decisions.md), implemented in [#18](https://github.com/itinera-dev/itinera-rs/issues/18). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 adds the events; the engine that emits them in the right order arrives in stages 3 to 7. Stage 6 emits every decision, after the hook points that could change it.

## API

- The decisions are the `EventBody` variants `StepRetrying`, `StepGivenUp`, `JourneySucceeded` and `JourneyFailed`.
- Who decided is part of what was decided, never a separate field that could disagree with it:
  - `StepRetrying` carries a `RetryCause` (`retriable failure`, `abnormal termination`) and is always decided by default, since no lifecycle asks for a retry.
  - `StepGivenUp` carries a `GiveUpCause` (`failure`, `abnormal termination`, `retries exhausted`, or `FailWorkflow` with its reason and a `DecidingHook<GiveUpHook>`, since only `on step retry` and `on step abnormal termination` give a step up).
  - `JourneySucceeded` carries the name of the policy whose `on step success` returned `FinishWorkflow`, the only hook that may, or `None` when no step was left.
  - `JourneyFailed` carries a `JourneyFailure`, whose `FailWorkflow` variant holds a `DecidingHook` naming any step hook.
- A `DecidingHook` is a policy and a step hook: only step hooks return lifecycles, so a workflow hook cannot be named as deciding.
- `hook_called` carries the `Lifecycle` returned, or `None`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Engine events carry keys, never data values | types: no engine event has a field of type `AnyValue`, apart from the details of a reason |
| A decision says who decided it | types: the deciding hook is held by the `FailWorkflow` causes and by `JourneySucceeded`; every other decision is by default |
| Only a hook that may return the deciding lifecycle is named | types: `DecidingHook` holds a `StepHook` or a `GiveUpHook`, and `JourneySucceeded` names only the policy of `on step success` |

## Tests

- Unit tests in `itinera-core/src/event.rs`: a step given up by `FailWorkflow` names `FailWorkflow` as its cause. Who decided needs no further test: the types hold it.
- Unit tests in `itinera-core/src/engine.rs`: exactly one decision follows each failed attempt, `step_given_up` comes before `on step failure`, and a lifecycle a scripted hook returns names its policy and hook in the decision it takes.

## Done when

Every scenario tagged `@proposal-0040` passes, and 40 is listed in `conformance.json`. Planned for stage 7.
