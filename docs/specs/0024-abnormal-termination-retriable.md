# 0024: Abnormal terminations are retried only when the step allows it

Tech spec for [proposal 0024](https://github.com/itinera-dev/spec/blob/main/proposals/0024-abnormal-termination-retriable.md), implemented in [#15](https://github.com/itinera-dev/itinera-rs/issues/15). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7. Stage 6 added the setting and the decision that reads it; stage 7 added the hooks around the decision, and what they may request about an abnormal termination.

## API

- **`itinera::step::StepDescriptor::abnormal_termination_retriable()`** lets an abnormal termination of the step be retried, within its retry budget. Without it, the step is given up at once with the cause `abnormal termination`.
- **`OnStepAbnormalTermination<W>`**, and `AsyncOnStepAbnormalTermination<W>` behind the `async` feature, is the hook called first after an abnormal termination. It may request the error, with `HookNeeds::error` or `optional_error`, and return `FailWorkflow` to give the step up.
- **The hooks after it** are told why they are called: `on step retry` with the `RetryCause::AbnormalTermination`, and `on step failure` with the `StepFailureCause::AbnormalTermination` or `RetriesExhausted`, each through `Requested::cause()`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| An abnormal termination is not retried unless the step descriptor says so | the descriptor's setting is false unless set, and the decision function gives the step up at once when it is not set |
| A retriable abnormal termination spends the same budget as a retriable failure | the decision function checks the one budget for both, and gives the step up with `retries exhausted` once it is spent |
| `on step abnormal termination` is called first, then the decision | the engine calls the hook before deciding, and a `FailWorkflow` there gives the step up without `on step failure` |
| The retry is recorded with the cause `abnormal termination` | the decision returns `RetryCause::AbnormalTermination`, which `step_retrying` carries and `on step retry` receives |
| When retries run out after an abnormal termination, the failure carries the last error and no reason | types: `LastFailure` holds either a reason or an error; see the tech spec of [0083](0083-what-failures-carry.md) |
| A hook's required request for a reason that does not exist aborts with `required data missing`; an optional one gets none | the engine, while it resolves the hook's requests: an abnormal termination has an error and no reason |

## Tests

- Unit tests in `itinera-core/src/engine/decision.rs`: the cases of `a_step_is_retried_only_for_a_retriable_end_while_its_budget_allows_another_attempt` for an abnormal termination the step does not retry, retries with budget left, and retries on the last attempt the budget allows; `retries_exhausted_carries_what_ended_the_last_attempt`.
- Unit tests in `itinera-core/src/engine.rs`: `an_abnormal_termination_is_retried_only_when_the_step_allows_it`; `an_abnormal_termination_is_retried_with_its_own_cause`; `step_hooks_are_called_after_each_attempt_in_order_around_the_step_decision`, where `on step abnormal termination` comes before `on step retry` or `on step failure`; `fail_workflow_from_a_hook_before_the_decision_gives_the_step_up_without_on_step_failure`, with a case for `on step abnormal termination`.
- Unit tests in `itinera-core/src/engine/hooks.rs`: `an_abnormal_termination_carries_no_contributions_to_its_hooks`, and the case `the_reason_of_an_abnormal_termination` of `a_required_request_without_a_value_aborts_the_journey_before_the_hook_runs`.

## Done when

Every scenario tagged `@proposal-0024` passes, and 24 is listed in `conformance.json`. Done in stage 7.
