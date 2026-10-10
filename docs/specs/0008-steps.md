# 0008: Steps

Tech spec for [proposal 0008](https://github.com/itinera-dev/spec/blob/main/proposals/0008-steps.md), implemented in [#10](https://github.com/itinera-dev/itinera-rs/issues/10). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals. Proposals 0024, 0056, 0057, 0064 and 0083 amend it, and their tech specs cover what they change.

Status: done in stage 7. Stage 5 built a new step for every attempt from its factory, with the inputs and handles it declares, and acted on its outcome and contributions. Stage 6 added retries, skips and abnormal terminations to the scan. Stage 7 added the hooks that receive an attempt's reason, cause, error and contributions.

## API

- **`itinera::step::StepFactory`**, and **`AsyncStepFactory`** behind the `async` feature, build a new step for every attempt. `needs()` declares, in a `StepNeeds`, the step's inputs, each an `Input<T>` or an `OptionalInput<T>`, and whether it takes a contributor and a reporter; `build` makes the step from `Resolved`, or fails. A closure returning `Result<Outcome, Error>` is the factory of a step that needs nothing.
- **`Step::run(self)`** returns `Result<Outcome, Error>`, and **`AsyncStep::run(self)`** a future of it, which can be written as an `async fn`. An `Err` is an abnormal termination of the attempt.
- **`itinera::step::Outcome`** is made with `success()`, `failure(reason)`, `retriable_failure(reason)`, `skipped()` or `skipped_because(reason)`. A `Reason` has a code, an optional message, set with `with_message`, and optional details, a value, set with `with_details`.
- **`itinera::journey::Contributor::contribute(key, value)`** takes any value, by value.
- **`StepDescriptor::new(name, factory)`**, or `new_async`, describes a step under a `StepName`, made with `step_name!`; `.retry_budget(n)` and `.abnormal_termination_retriable()` set how it is retried.
- **What a step's hooks receive about its attempt.** `on step failure` takes its cause with `Requested::cause()`, a `StepFailureCause`: `failure`, `abnormal termination` or `retries exhausted`. `on step retry` takes its cause the same way, a `RetryCause`: `retriable failure` or `abnormal termination`. Both may request the reason, with `HookNeeds::reason` or `optional_reason`, and both, with `on step abnormal termination`, may request the error, with `HookNeeds::error` or `optional_error`; the hook takes them with `Requested::reason`, `optional_reason`, `error` and `optional_error`. Every step hook may take what the attempt contributed with `Requested::from_step`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Every attempt builds a new step, and nothing carries over from one attempt to the next | types: `run(self)` consumes the step; the engine calls the factory for every attempt |
| Inputs are resolved outside the step, which cannot reach the adapter, the data bag or the resolver | types: the factory receives only `Resolved`, which holds values already resolved |
| A step does not know its own name | types: neither `StepFactory::build` nor `Step::run` receives it; a step policy may take it with `Requested::step_name()` |
| A step that cannot be built aborts the journey, without entering the retry loop | the engine: an input that cannot be resolved, an input adapter that fails or a factory that fails ends the attempt with an abort before the step runs, and no decision follows |
| An error escaping a running step ends the attempt with an abnormal termination | types: `run` returns an `Err`, which the engine reports with `step_abnormal_termination`; a panic is never caught (proposal 0054) |
| An outcome states what happened: success, failure or skipped; a failure is not retriable unless the step says so | types: `Outcome` is made only by its constructors; `failure` is not retriable, `retriable_failure` is |
| A failure carries a reason; a skip may | types: `failure` and `retriable_failure` take a `Reason`; `skipped_because` takes one, `skipped` does not |
| The failure hook may request the reason and the cause, the abnormal termination hook the error, and the retry hook the cause and the reason or error | types: `HookNeeds` offers `reason` only to `on step failure` and `on step retry`, and `error` to those and to `on step abnormal termination`; `cause()` exists only for `on step failure` and `on step retry` |
| When the retry budget is spent, the failure hook is called with the cause `retries exhausted` | the engine's decision function |
| A skipped step is out of the flow: no hook runs, and no lifecycle can follow | the engine emits `step_skipped` and moves to the next step without calling any hook |
| A skipped step's contributions are discarded, and an event records it | the engine emits `contributions_discarded` and commits nothing |
| Only the step decides to skip | types: no lifecycle a hook may return skips a step |
| The same key twice in one attempt keeps the last value | the attempt's contributions keep each key once, with its last value |
| Contributions are visible to the attempt's hooks whatever its outcome, and committed only when it succeeds; an abnormal termination carries none | the engine keeps an attempt's contributions for its hooks, commits them only after `step_succeeded`, and gives the hooks of an abnormal termination none |
| A committed contribution may replace a key in the data bag, and an event records it | `WorkflowInstance::commit` answers `Committed::Overwritten`, from which the engine emits `data_overwritten` |
| The executor never interrupts a running step | the engine awaits each step to its end, and never cancels, sleeps or sets a timer |
| A step's name is non-empty, case-sensitive and unique in its workflow | `StepName::new` panics on an empty name, and `step_name!` makes an empty literal a compile error; names compare as exact text; `build()` refuses `duplicate step name` |

## Tests

- Unit tests in `itinera-core/src/engine.rs`: `a_step_whose_factory_fails_aborts_the_journey_as_it_could_not_be_built`; `without_a_retry_budget_a_failed_attempt_is_the_steps_last_and_fails_the_journey`; `a_retriable_failure_is_attempted_again_while_the_budget_allows`; `a_step_whose_budget_is_spent_is_given_up_with_its_last_failure`; `a_skipped_step_discards_its_contributions_and_the_journey_goes_on`; `step_hooks_are_called_after_each_attempt_in_order_around_the_step_decision`, whose case for a skip calls no hook; `a_successful_attempts_contributions_are_committed_in_order_reporting_what_they_overwrote`; `an_attempt_that_does_not_succeed_commits_nothing`.
- Unit tests in `itinera-core/src/engine/decision.rs`: `a_step_is_retried_only_for_a_retriable_end_while_its_budget_allows_another_attempt` and `retries_exhausted_carries_what_ended_the_last_attempt`.
- Unit tests in `itinera-core/src/engine/hooks.rs`: `a_step_hook_receives_what_it_requests_from_the_step_and_the_workflow_before_it_runs`, where `on step failure` receives the failed attempt's contribution, its reason and its cause, and the contribution never reaches the data bag; `an_abnormal_termination_carries_no_contributions_to_its_hooks`.
- Unit tests in `itinera-core/src/journey/contributor.rs`: `a_key_contributed_twice_keeps_its_last_value_in_its_first_place`.
- Unit tests in `itinera-core/src/step.rs`: `a_step_name_cannot_be_empty`; in `itinera-core/src/workflow/violation.rs`, the case `two_steps_with_one_name` of `a_workflow_put_together_wrongly_is_refused_with_its_violation`.

## Done when

Every scenario tagged `@proposal-0008` passes, and 8 is listed in `conformance.json`. Done in stage 7.
