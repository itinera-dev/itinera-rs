# 0083: What a failure, an abort and a refusal carry

Tech spec for [proposal 0083](https://github.com/itinera-dev/spec/blob/main/proposals/0083-what-failures-carry.md), implemented in [#46](https://github.com/itinera-dev/itinera-rs/issues/46). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 shapes the events, stage 3 the result and refusals, and stage 7 what hooks may request.

## API

- **`journey_failed`** is `EventBody::JourneyFailed`, with the step's name and a `JourneyFailure`, one variant per cause, each holding exactly what that cause carries:
  - `Failure` holds the step's `Reason`;
  - `RetriesExhausted` holds a `LastFailure`: the `Reason` of a retriable failure, or the message of the error of an abnormal termination;
  - `AbnormalTermination` holds the error's message;
  - `FailWorkflow` holds the `DecidingHook` and the `Reason` it gave.
- `JourneyFailure::cause()` gives the plain `FailureCause`.
- **`journey_aborted`** is `EventBody::JourneyAborted`, holding a `JourneyAbort` with one variant per abort reason, each holding exactly what that reason carries:
  - `StepCouldNotBeBuilt`, `HookFailed` and `ReporterFailed` hold the error's message, and the step's name where there is one;
  - `PolicyCouldNotBeBuilt` holds the step's name, the policy and the error's message;
  - `RequiredDataMissing` holds a `MissingData`: a key and its `Requester`, or a step hook's request for the failure's reason or the error, which have no key;
  - `WrongType` holds the key and the `Requester`;
  - a `Requester` names the step except for a workflow hook, and neither reason carries an error.
- `JourneyAbort::reason()`, `step()` and `error()` give the plain `AbortReason`, the step's name and the error's message.
- Neither carries a `StepAttempt`, only the step's name.
- Events carry an error only as its `Display` text, never an `itinera::error::Error`.
- **The result** mirrors the events, holding the error itself where they hold its message. `itinera::journey::Failure` has one variant per cause: `Failure` and `FailWorkflow` hold the `Reason`, `RetriesExhausted` a `LastFailure<Error>`, and `AbnormalTermination` the `Error`. `itinera::journey::Abort` has one variant per abort reason: `StepCouldNotBeBuilt`, `HookFailed` and `ReporterFailed` hold the `Error`, `PolicyCouldNotBeBuilt` the policy's name and the `Error`, `RequiredDataMissing` a `MissingData`, and `WrongType` the key and the `Requester`. The result's `MissingData` and `Requester` are those of events without the step's name, since the result names no step. `Failure::cause()` and `Abort::reason()` give the plain `FailureCause` and `AbortReason`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A failure carries a reason, an error, or both, exactly as its cause says | types: the variants of `JourneyFailure` and `LastFailure` |
| Events carry an error only as its message | types: the error fields of events are `String` |
| `journey_failed` and `journey_aborted` carry no attempt number | types: they hold a step name, not a `StepAttempt` |
| An abort carries an error exactly when custom code caused it by failing | types: the variants of `JourneyAbort` and `Abort`; in Rust a constructor, input adapter or policy can only fail by returning an error |
| The result carries the error itself, not only its message | types: the error fields of `Failure` and `Abort` are `Error` |

## Tests

- Unit tests in `itinera-core/src/event.rs`: a journey failure names its cause; an abort while data was resolved names the step it was for but no error; a required request for a missing reason names the step the hook acts on; an abort by a workflow hook's request names no step; an abort caused by failing code carries the error's message.
- Unit tests in `itinera-core/src/journey.rs`: a failure names its cause, and an abort its reason.

## Done when

Every scenario tagged `@proposal-0083` passes, and 83 is listed in `conformance.json`. Planned for stage 7.
