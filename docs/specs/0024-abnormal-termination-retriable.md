# 0024: Abnormal terminations are retried only when the step allows it

Tech spec for [proposal 0024](https://github.com/itinera-dev/spec/blob/main/proposals/0024-abnormal-termination-retriable.md), implemented in [#15](https://github.com/itinera-dev/itinera-rs/issues/15). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 6 adds the setting and the decision that reads it.

## API

- **`itinera::step::StepDescriptor::abnormal_termination_retriable()`** lets an abnormal termination of the step be retried, within its retry budget. Without it, the step is given up at once with the cause `abnormal termination`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| An abnormal termination is not retried unless the step descriptor says so | the descriptor's setting is false unless set, and the decision function gives the step up at once when it is not set |
| A retriable abnormal termination spends the same budget as a retriable failure | the decision function checks the one budget for both, and gives the step up with `retries exhausted` once it is spent |
| `on step abnormal termination` is called first, then the decision | the engine calls the hook point before deciding, and a `FailWorkflow` there gives the step up without `on step failure` |
| The retry is recorded with the cause `abnormal termination` | the decision returns `RetryCause::AbnormalTermination`, which `step_retrying` carries |

## Tests

- Unit tests in `itinera-core/src/engine/decision.rs`: the cases for an abnormal termination the step does not retry, retries with budget left, and retries on the last attempt the budget allows.
- Unit tests in `itinera-core/src/engine.rs`: an abnormal termination is retried only when the step allows it, and with its own cause; `on step abnormal termination` comes before `on step retry`.

## Done when

Every scenario tagged `@proposal-0024` passes, and 24 is listed in `conformance.json`. Planned for stage 7, when hooks can be written.
