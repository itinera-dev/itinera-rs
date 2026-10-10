# 0009: Running a workflow

Tech spec for [proposal 0009](https://github.com/itinera-dev/spec/blob/main/proposals/0009-running-a-workflow.md), implemented in [#11](https://github.com/itinera-dev/itinera-rs/issues/11). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7. Stage 3 ran the steps in order; stage 5 built each step for its attempt and acted on its outcome. Stage 6 added the scan with its attempts, retries and decisions, and the hook points around the decisions. Stage 7 calls the policies' hooks there, whose lifecycles override the default.

## API

- **`itinera::step::StepDescriptor::retry_budget(n)`** sets how many retries a step allows after its first attempt, as a `u16`, 0 unless set. A step with a budget of `n` is attempted at most `n + 1` times.
- **Step statuses are not public**, and the result holds none: the engine's scan keeps them to itself, and they can be derived from the event stream.
- **The hooks** called around each decision, and the lifecycles that override it, are described in the tech spec of [0010](0010-hooks-lifecycles-and-roles.md).
- Everything else a journey shows is in its events and its result, described in the tech specs of 0040 and 0065.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Steps run in the order of their descriptors, and nothing reorders or skips them | the scan goes through the descriptor's steps in order, and only the step's own outcome ends it as skipped |
| Run-once: a step that succeeded, failed or skipped itself never runs again | the scan moves to the next step once a step is done, and the journey ends when a step fails |
| Attempts are counted from 1, and a step with a budget of N is attempted at most N + 1 times | types: attempts are `NonZeroU32`, starting at 1; the decision allows another attempt only while the attempt's number is at most the budget |
| Attempt numbers never overflow | types: the budget is a `u16`, so the last attempt it allows is 65,536 |
| A retriable failure, or a retriable abnormal termination, is retried while the budget allows | the pure decision function in the engine, then `on step retry`, which may give the step up by returning `FailWorkflow`, then `step_retrying` |
| A retry happens immediately | the engine never waits: it only awaits the workflow's own parts |
| A failure that is not retriable, or a spent budget, gives the step up and fails the journey | the decision function, then `step_given_up`, then `on step failure`, then `on workflow failure`, then `journey_failed` |
| A hook's `FinishWorkflow` or `FailWorkflow` ends the journey at once, and the steps left do not run | the engine ends the scan with the hook's lifecycle |
| A skipped step is passed over, with no hook and no decision | the engine emits `contributions_discarded` and moves to the next step |
| Building fails: the step is aborted and never retried | the engine aborts the journey before the step runs, without a decision |

## Tests

- Unit tests in `itinera-core/src/engine/decision.rs`: `a_step_is_retried_only_for_a_retriable_end_while_its_budget_allows_another_attempt`, with a named case for each end and budget, the largest included; `retries_exhausted_carries_what_ended_the_last_attempt`.
- Unit tests in `itinera-core/src/engine.rs`: `steps_run_in_the_order_they_were_added`; `a_retriable_failure_is_attempted_again_while_the_budget_allows`, each attempt numbered in turn; `a_step_whose_budget_is_spent_is_given_up_with_its_last_failure`; `a_skipped_step_discards_its_contributions_and_the_journey_goes_on`; `step_hooks_are_called_after_each_attempt_in_order_around_the_step_decision`; and the lifecycles a policy's hook returns, which finish or fail the journey as chapter 6 says: `finish_workflow_from_on_step_success_succeeds_the_journey_without_the_steps_left`, `fail_workflow_from_on_step_success_fails_the_journey_after_committing_the_contributions`, `fail_workflow_from_a_hook_before_the_decision_gives_the_step_up_without_on_step_failure` and `fail_workflow_from_on_step_failure_gives_the_journey_its_reason_after_the_step_is_given_up`.
- Unit tests in `itinera-core/src/executor.rs`: `the_step_runs_once`.

## Done when

Every scenario tagged `@proposal-0009` passes, and 9 is listed in `conformance.json`. Done in stage 7.
