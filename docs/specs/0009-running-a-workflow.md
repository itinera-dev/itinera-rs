# 0009: Running a workflow

Tech spec for [proposal 0009](https://github.com/itinera-dev/spec/blob/main/proposals/0009-running-a-workflow.md), implemented in [#11](https://github.com/itinera-dev/itinera-rs/issues/11). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 3 runs the steps in order; stage 5 builds each step for its attempt and acts on its outcome. Stage 6 adds the scan with its attempts, retries and decisions, and the hook points where stage 7 calls the hooks.

## API

- **`itinera::step::StepDescriptor::retry_budget(n)`** sets how many retries a step allows after its first attempt, as a `u16`, 0 unless set. A step with a budget of `n` is attempted at most `n + 1` times.
- **Step statuses are not public**, and the result holds none: the engine's scan keeps them to itself, and they can be derived from the event stream.
- Everything else a journey shows is in its events and its result, described in the tech specs of 0040 and 0065.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Steps run in the order of their descriptors, and nothing reorders or skips them | the scan goes through the descriptor's steps in order, and only the step's own outcome ends it as skipped |
| Run-once: a step that succeeded, failed or skipped itself never runs again | the scan moves to the next step once a step is done, and the journey ends when a step fails |
| Attempts are counted from 1, and a step with a budget of N is attempted at most N + 1 times | types: attempts are `NonZeroU32`, starting at 1; the decision allows another attempt only while the attempt's number is at most the budget |
| Attempt numbers never overflow | types: the budget is a `u16`, so the last attempt it allows is 65,536 |
| A retriable failure, or a retriable abnormal termination, is retried while the budget allows | the pure decision function in the engine, then `on step retry`, which may give the step up, then `step_retrying` |
| A retry happens immediately | the engine never waits: it only awaits the workflow's own parts |
| A failure that is not retriable, or a spent budget, gives the step up and fails the journey | the decision function, then `step_given_up`, then `on step failure`, then `journey_failed` |
| A skipped step is passed over, with no hook and no decision | the engine emits `contributions_discarded` and moves to the next step |
| Building fails: the step is aborted and never retried | the engine aborts the journey before the step runs, without a decision |

## Tests

- Unit tests in `itinera-core/src/engine/decision.rs`: a step is retried only for a retriable end while its budget allows another attempt, with a named case for each end and budget, the largest included; retries exhausted carries what ended the last attempt.
- Unit tests in `itinera-core/src/engine.rs`: a retriable failure is attempted again while the budget allows, each attempt numbered in turn; a step whose budget is spent is given up with its last failure; step hooks are called after each attempt in order around the step decision; and the lifecycles a scripted hook may return finish or fail the journey as chapter 6 says.

## Done when

Every scenario tagged `@proposal-0009` passes, and 9 is listed in `conformance.json`. Planned for stage 7, when hooks can be written.
