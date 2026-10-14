# 0106: The retry budget alone decides retries

Tech spec for [proposal 0106](https://github.com/itinera-dev/spec/blob/main/proposals/0106-retry-budget-alone.md), implemented in [#136](https://github.com/itinera-dev/itinera-rs/issues/136). It amends [0003](0003-rescue-steps.md), [0008](0008-steps.md), [0009](0009-running-a-workflow.md), [0010](0010-hooks-lifecycles-and-roles.md), [0011](0011-events.md), [0024](0024-abnormal-termination-retriable.md), [0040](0040-events-facts-and-decisions.md) and [0083](0083-what-failures-carry.md). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done.

## API

- **`itinera::step::Outcome`** is made with `success()`, `failure(reason)`, `skipped()` or `skipped_because(reason)`, and read back with `is_success()`, `is_failure()`, `is_skipped()` and `reason()`. `retriable_failure` and `is_retriable` are gone, and so are the same two on **`RescueOutcome`**, which keeps `success()` and `failure(reason)`.
- **The shorthands** `DescribedStep::fail_retriably` and `DescribedRescueStep::fail_retriably` are gone: a step written with `#[step]` or `#[rescue_step]` writes `Self::fail(reason)`.
- **`StepDescriptor::no_retry_on_abnormal_termination()`** replaces `abnormal_termination_retriable()`. Without it, an abnormal termination is retried within the retry budget, as a failure is; with it, an abnormal termination gives the step up at once, whatever its budget. On `#[with_step(...)]`, the flag `no_retry_on_abnormal_termination` replaces `abnormal_termination_retriable`.
- **`itinera::policy::RetryCause`** is `Failure`, which displays `failure` and replaces `RetriableFailure`, or `AbnormalTermination`. **`itinera::event::RescueCause`** is `Failure` or `AbnormalTermination`: `RetriableFailure` is gone. `StepFailureCause` and `GiveUpCause` keep their variants.
- **`EventBody::StepFailed`** holds the step's attempt and the reason, and no `retriable` field.
- **Scripted runs**: `will_fail_retriably` and `step::Ending::RetriableFailure` are gone; a script writes `will_fail(reason)` for every failure.

## The setting's name in Rust

The specification names the setting `retry on abnormal termination`, true unless the descriptor says otherwise. A builder method that sets it to true would change nothing, so the descriptor offers only the way to turn it off, named for what it does: `no_retry_on_abnormal_termination()`, and the flag of the same name on `#[with_step]`. A step that keeps the default writes nothing, as with a retry budget of 0.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A failure carries a reason and nothing else, and nothing a step reports asks for a retry | types: `Outcome` and `RescueOutcome` have one failure, which holds a `Reason`; `StepFailed` has no `retriable` field |
| After a failure, or an abnormal termination while `retry on abnormal termination` is on, the step is retried while its budget allows another attempt, unless a hook returns `FailWorkflow` | `decide` in `engine/decision.rs` returns `Decision::Retry` while `StepDescriptor::budget_allows_after` holds; `engine/retry.rs` then calls `on step retry`, whose `FailWorkflow` gives the step up, and otherwise emits `step_retrying` |
| Failures and abnormal terminations spend the same budget | `within_budget` in `engine/decision.rs` checks the one budget for both causes |
| A budget of 0 means no retries; the budget is per step, explicit and 0 by default | `StepDescriptor::new` sets it to 0, and only `retry_budget(n)` changes it; the workflow builder has no default of its own |
| Once a budget greater than 0 is spent, the step is given up with `retries exhausted`, and `on step retry` is not called | `within_budget` returns `Decision::GiveUp(StepFailureCause::RetriesExhausted)` when `allows_retries()` holds; the engine calls `on step retry` only for `Decision::Retry` |
| A step with a budget of 0 is given up with the cause of its only attempt, `failure` or `abnormal termination` | `within_budget` returns `StepFailureCause::Failure` or `StepFailureCause::AbnormalTermination` when `allows_retries()` does not hold |
| `retry on abnormal termination` belongs to each step descriptor, true unless it says otherwise, with no workflow-wide default | `StepDescriptor::new` sets it to true, and only `no_retry_on_abnormal_termination()` turns it off |
| With the setting off, an abnormal termination gives the step up at once with the cause `abnormal termination`, whatever its budget | `decide` reads the setting before the budget, and gives the step up when it is off |
| An abnormal termination calls `on step abnormal termination` before the decision | `engine/outcome.rs` calls the hook before `failed`, and its `FailWorkflow` gives the step up with that hook as its cause |
| A skip calls no hook and records no step decision | unchanged: `engine/outcome.rs` emits `step_skipped` and goes on |
| A step with a rescue step is rescued after its first failed attempt, whatever its outcome or its setting | `decide` checks the rescue before how the attempt ended or the setting; admission refuses a rescued step with a retry budget, `rescued step retried` |
| After its rescue, the step's attempt 2 is given up with its own cause | the rescued step has a budget of 0, so `within_budget` gives it up with `failure` or `abnormal termination` |
| A rescue step that fails is given up with `failure` or `abnormal termination` | the crate describes the rescue step with a budget of 0, so it follows the same rule |
| `on step retry` and `step_retrying` carry `failure` or `abnormal termination`, and so does `step_rescuing` | types: `RetryCause` and `RescueCause` have only those two variants |
| `retries exhausted` carries the reason of the last attempt if it reported a failure, and its error if it ended in an abnormal termination | `Failed::into_last` makes a `LastFailure` holding one or the other |

## Tests

- Unit tests in `itinera-core/src/engine/decision.rs`: `a_step_is_retried_while_its_budget_allows_another_attempt`, with a case for a failure and for an abnormal termination with budget left, on the last attempt the budget allows and without a budget, for an abnormal termination of a step that does not retry them, and for the largest budget; `a_step_with_a_rescue_step_is_rescued_after_its_first_attempt_however_it_failed`, with a case for an abnormal termination of a step that does not retry them; `a_rescued_step_is_given_up_after_the_attempt_that_follows_its_rescue`; `retries_exhausted_carries_what_ended_the_last_attempt`.
- Unit tests in `itinera-core/src/engine/retry/tests.rs`: `without_a_retry_budget_a_failed_attempt_is_the_steps_last_and_fails_the_journey`; `a_failure_is_attempted_again_while_the_budget_allows`, where `step_retrying` carries `failure`; `a_step_whose_budget_is_spent_is_given_up_with_its_last_failure`; `an_abnormal_termination_is_retried_unless_the_step_turns_it_off`; `an_abnormal_termination_is_retried_with_its_own_cause`; `fail_workflow_from_a_hook_before_the_decision_gives_the_step_up_without_on_step_failure`; `step_hooks_are_called_after_each_attempt_in_order_around_the_step_decision`, with a case for a failure with budget left, with the budget spent and without a budget, and for an abnormal termination the step does not retry, with budget left and without a budget.
- Unit tests in `itinera-core/src/engine/rescue/tests.rs`: `a_failed_first_attempt_is_rescued_then_the_step_is_attempted_again`, with the cases `an_abnormal_termination_the_step_would_retry` and `an_abnormal_termination_the_step_does_not_retry`; `a_rescue_step_that_does_not_succeed_fails_the_journey_and_the_step_is_not_attempted_again`, with the cause `failure` or `abnormal termination`.
- Unit tests in `itinera-core/src/step/outcome.rs`: `an_outcome_says_what_it_is_and_why`. In `policy.rs`: `hooks_and_causes_display_as_the_specification_writes_them`, with the case `retry_after_a_failure`. In `event/decision.rs`: `names_display_as_the_specification_writes_them`. In `event/body/tests.rs`: `every_kind_of_event_has_its_own_kind`, with `step_failed` and no flag.
- Unit tests in `itinera-core/src/instance/scripted/checks.rs`: `a_failure_with_budget_left_needs_the_next_attempt_scripted`; `an_abnormal_termination_the_step_retries_needs_the_next_attempt_scripted`, whose step keeps the default setting; `a_rescued_step_that_can_fail_on_attempt_1_needs_attempt_2_and_its_rescue_step_scripted`, whose step does not retry abnormal terminations; `no_attempt_is_needed_after_one_the_step_cannot_be_attempted_again_after`.
- Equivalence tests in `crates/itinera/tests/macros/workflow.rs`: `a_workflow_the_macros_declare_lists_and_runs_as_one_declared_with_the_builder`, whose `charge` step is listed with `no_retry_on_abnormal_termination` and retried after a failure. In `rescue.rs`: `a_rescue_step_the_macros_describe_lists_and_runs_as_one_written_for_the_builder`.
- The compile-fail test `an_unknown_step_setting` in `crates/itinera/tests/ui/workflow/`, whose message lists `no_retry_on_abnormal_termination`.
- The runner: the sentence `step "<step>" does not retry abnormal terminations`, in `sentences/building.rs`, replaces the one that made an abnormal termination retriable; the outcome `retriable failure` and the event column `retriable` are gone. `tests/cases/given/every-given-sentence.feature` uses the new sentence, and in `trace/tests.rs`, `an_empty_cell_is_not_compared` checks a cell `step_failed` leaves empty.

## Done when

Every scenario tagged `@proposal-0106` passes, and 106 is listed in `conformance.json`.
