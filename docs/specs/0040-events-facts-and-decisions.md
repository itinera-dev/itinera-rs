# 0040: Events record facts and decisions

Tech spec for [proposal 0040](https://github.com/itinera-dev/spec/blob/main/proposals/0040-events-facts-and-decisions.md), implemented in [#18](https://github.com/itinera-dev/itinera-rs/issues/18). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7. Stage 1 added the events, and stages 3 to 6 the engine that emits them in order. Stage 6 emitted every decision after the hook points that could change it, and stage 7 calls the hooks there, whose lifecycles the decisions name.

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

- Unit tests in `itinera-core/src/event.rs`: `a_step_given_up_by_fail_workflow_names_fail_workflow_as_its_cause`. Who decided needs no further test: the types hold it.
- Unit tests in `itinera-core/src/engine.rs`: `step_hooks_are_called_after_each_attempt_in_order_around_the_step_decision`, where exactly one decision follows each failed attempt and `step_given_up` comes before `on step failure`; and the tests of the lifecycles a policy's hook returns, which name its policy and hook in the decision they take: `finish_workflow_from_on_step_success_succeeds_the_journey_without_the_steps_left`, `fail_workflow_from_on_step_success_fails_the_journey_after_committing_the_contributions`, `fail_workflow_from_a_hook_before_the_decision_gives_the_step_up_without_on_step_failure` and `fail_workflow_from_on_step_failure_gives_the_journey_its_reason_after_the_step_is_given_up`.

## Done when

Every scenario tagged `@proposal-0040` passes, and 40 is listed in `conformance.json`. Done in stage 7.
