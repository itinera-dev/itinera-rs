# 0083: What a failure, an abort and a refusal carry

Tech spec for [proposal 0083](https://github.com/itinera-dev/spec/blob/main/proposals/0083-what-failures-carry.md), implemented in [#46](https://github.com/itinera-dev/itinera-rs/issues/46). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 shapes the events; the result and refusals arrive in stage 3, and what hooks may request in stage 7.

## API

- **`journey_failed`** is `EventBody::JourneyFailed`, with the step's name and a `JourneyFailure`, one variant per cause, each holding exactly what that cause carries:
  - `Failure` holds the step's `Reason`;
  - `RetriesExhausted` holds a `LastFailure`: the `Reason` of a retriable failure, or the message of the error of an abnormal termination;
  - `AbnormalTermination` holds the error's message;
  - `FailWorkflow` holds the `DecidingHook` and the `Reason` it gave.
- `JourneyFailure::cause()` gives the plain `FailureCause`. A `JourneyFailure` serializes as `cause`, `reason`, `error` and `decided_by`, with `null` for what the cause does not carry.
- **`journey_aborted`** is `EventBody::JourneyAborted`, with the step's name if any, the `AbortReason`, its `AbortDetails`, and the error's message when failing custom code caused the abort.
- Neither carries a `StepAttempt`, only the step's name.
- Events carry an error only as its `Display` text, never an `itinera::Error`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A failure carries a reason, an error, or both, exactly as its cause says | types: the variants of `JourneyFailure` and `LastFailure` |
| Events carry an error only as its message | types: the error fields of events are `String` |
| `journey_failed` and `journey_aborted` carry no attempt number | types: they hold a step name, not a `StepAttempt` |
| An abort carries an error only when custom code caused it by failing | the executor, from stage 3 |

## Tests

- Unit tests in `itinera-core/src/event.rs`: a journey failed after retries carries the last attempt's error message; a journey failed by `FailWorkflow` carries the reason and the hook.

## Done when

Every scenario tagged `@proposal-0083` passes, and 83 is listed in `conformance.json`. Planned for stage 7.
