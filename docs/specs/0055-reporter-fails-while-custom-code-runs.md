# 0055: A reporter that fails while a step or hook runs

Tech spec for [proposal 0055](https://github.com/itinera-dev/spec/blob/main/proposals/0055-reporter-fails-while-custom-code-runs.md), implemented in [#23](https://github.com/itinera-dev/itinera-rs/issues/23). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals. Proposal 0081 amends it: the event a reporter failed on is not delivered to the reporters after it, as the tech spec of [0081](0081-reporter-failure-stops-delivery.md) says.

Status: done in stage 7. Stage 5 added the step reporters, which deliver each event before they return and end the step when a reporter fails. Stage 7 gave hooks their own reporters, which follow the same rule.

## API

- **`itinera::step::StepReporter`** and **`itinera::policy::HookReporter`**, with **`AsyncStepReporter`** and **`AsyncHookReporter`** behind the `async` feature, emit a step's or hook's own events with `info`, `warning` and `error`, each with a `_with` form that adds data. A step takes one from `Resolved::reporter`, and a hook from `Requested::reporter`, once it has declared it.
- **Each call returns `Result<(), itinera::error::Interrupted>`**, or a future of it, once every reporter has the event. `Interrupted` is the executor's signal that a reporter failed during that delivery; the step or hook propagates it with `?`, which converts it into an `itinera::error::Error`. It is `#[non_exhaustive]`, so only itinera makes one.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| An event a step or hook emits is delivered before its emit call returns | the handles deliver through the journey's dispatcher before returning; an asynchronous handle's future completes only once every reporter has the event |
| A reporter that fails during that delivery aborts the journey with `reporter failed`, and the abort is recorded at once | the engine records the abort as soon as the delivery fails, before the call returns |
| The emit call then ends the step or hook, which propagates the executor's error | types: the call returns `Interrupted`, and `?` propagates it as the step's or hook's `Err` |
| The abort stands whatever the step or hook does afterwards: nothing more it emits is delivered, and its outcome, lifecycle and contributions are ignored, with no outcome fact, no `hook_called` and no commit | the engine checks for the recorded abort when the step or hook returns, before it looks at what it returned; the handles deliver nothing more once the abort is recorded |
| It is not an abnormal termination, and for a hook the reason is `reporter failed`, not `hook failed` | the engine checks for the recorded abort before it treats an `Err` as an abnormal termination or a hook's failure |
| `journey_aborted` follows, naming the step, or the step a step hook acts on | the engine aborts the journey with the step it was running |
| What ends the step should be impossible to catch by name | Rust has no exceptions: `Interrupted` is a returned value, which a step or hook could ignore, so the recorded abort is what makes the outcome certain |

## Tests

- Unit tests in `itinera-core/src/engine.rs`: `a_reporter_failing_on_a_step_event_aborts_the_journey_whatever_the_step_does_next`; `a_reporter_failing_on_an_event_emitted_while_the_step_is_built_aborts_the_journey`; with the `async` feature, `an_asynchronous_step_awaits_its_own_events` and `a_reporter_failing_on_an_asynchronous_steps_event_aborts_the_journey`.
- Unit tests in `itinera-core/src/engine/hooks.rs`: `a_reporter_failing_on_a_hooks_event_aborts_the_journey_whatever_the_hook_does_next`.
- Unit tests in `itinera-core/src/executor.rs`: `journey_aborted_names_the_step_during_which_a_reporter_failed`.

## Done when

Every scenario tagged `@proposal-0055` passes, and 55 is listed in `conformance.json`. Done in stage 7.
