# 0083: What a failure, an abort and a refusal carry

Tech spec for [proposal 0083](https://github.com/itinera-dev/spec/blob/main/proposals/0083-what-failures-carry.md), implemented in [#46](https://github.com/itinera-dev/itinera-rs/issues/46). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7. Stage 1 shaped the events, stage 3 the result and refusals, and stage 7 what hooks may request about a failure.

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
- **What a step hook may request** follows the same rule. `on step failure` and `on step retry` may request the reason, with `HookNeeds::reason` or `optional_reason`; they and `on step abnormal termination` may request the error, with `HookNeeds::error` or `optional_error`. The hook takes them with `Requested::reason`, which gives a `Reason`, `optional_reason`, `error`, which gives a `&Error`, and `optional_error`. A required request for one the attempt does not have aborts the journey with `required data missing`, holding `MissingData::Reason` or `MissingData::Error` with the policy and the hook, and, in events, the step.
- **A refusal** carries the error of the custom code that caused it: `Refusal::WorkflowPolicy` with the policy's name and its error, `Refusal::DispatcherFactory` and `Refusal::Dispatcher`.
- **The result** mirrors the events, holding the error itself where they hold its message. `itinera::journey::Failure` has one variant per cause: `Failure` and `FailWorkflow` hold the `Reason`, `RetriesExhausted` a `LastFailure<Error>`, and `AbnormalTermination` the `Error`. `itinera::journey::Abort` has one variant per abort reason: `StepCouldNotBeBuilt`, `HookFailed` and `ReporterFailed` hold the `Error`, `PolicyCouldNotBeBuilt` the policy's name and the `Error`, `RequiredDataMissing` a `MissingData`, and `WrongType` the key and the `Requester`. The result's `MissingData` and `Requester` are those of events without the step's name, since the result names no step. `Failure::cause()` and `Abort::reason()` give the plain `FailureCause` and `AbortReason`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A failure carries a reason, an error, or both, exactly as its cause says | types: the variants of `JourneyFailure` and `LastFailure` |
| Events carry an error only as its message | types: the error fields of events are `String` |
| `journey_failed` and `journey_aborted` carry no attempt number | types: they hold a step name, not a `StepAttempt` |
| An abort carries an error exactly when custom code caused it by failing | types: the variants of `JourneyAbort` and `Abort`; in Rust a constructor, input adapter or policy can only fail by returning an error |
| The result carries the error itself, not only its message | types: the error fields of `Failure` and `Abort` are `Error` |
| A refusal caused by failing custom code carries its error | types: each `Refusal` variant holds the `Error` |
| A step hook may request the reason and the error; a reason exists when the attempt reported a failure, and an error when it ended in an abnormal termination, whatever the hook and the cause | types: `HookNeeds` offers `reason` to `on step failure` and `on step retry`, and `error` to those and `on step abnormal termination`; the engine answers from how the attempt ended, so `on step failure` after retries exhausted by an abnormal termination gets the last error and no reason |
| A required request for a reason or an error that does not exist aborts with `required data missing`; an optional one is answered absent | the engine, while it resolves the hook's requests before it runs |

## Tests

- Unit tests in `itinera-core/src/event.rs`: `a_journey_failure_names_its_cause`; `an_abort_while_data_was_resolved_names_the_step_it_was_for_but_no_error`; `a_required_request_for_a_missing_reason_names_the_step_the_hook_acts_on`; `an_abort_by_a_workflow_hooks_request_names_no_step`; `an_abort_caused_by_failing_code_carries_the_errors_message`.
- Unit tests in `itinera-core/src/journey.rs`: `a_failure_names_its_cause` and `an_abort_names_its_reason`.
- Unit tests in `itinera-core/src/engine/hooks.rs`: `a_step_hook_receives_what_it_requests_from_the_step_and_the_workflow_before_it_runs`, where `on step failure` receives the reason of a failure and no error; `a_required_request_without_a_value_aborts_the_journey_before_the_hook_runs`, with the cases `the_reason_of_an_abnormal_termination` and `the_error_of_a_failure`; `a_reason_declared_again_replaces_the_earlier_declaration`; `a_hook_that_fails_aborts_the_journey_with_its_error`; `a_step_policy_that_cannot_be_built_aborts_the_journey_before_the_steps_inputs`.
- Unit tests in `itinera-core/src/executor.rs`: `a_workflow_policy_that_cannot_be_built_refuses_the_journey_before_its_dispatcher`, whose refusal names the policy.

## Done when

Every scenario tagged `@proposal-0083` passes, and 83 is listed in `conformance.json`. Done in stage 7.
