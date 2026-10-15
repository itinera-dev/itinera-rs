# 0003: Rescue steps

Tech spec for [proposal 0003](https://github.com/itinera-dev/spec/blob/main/proposals/0003-rescue-steps.md), chapter 7 of the specification, implemented in [#104](https://github.com/itinera-dev/itinera-rs/issues/104). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done.

What chapter 7 says about services, which a rescue step may request and which are released around its attempt, belongs to services, implemented in stage 10 of the plan, and is in the tech spec of [0095](0095-services-provided-to-steps.md). Proposal 0098 amends this one: input adapters and service providers may be attached to a rescue step by its name, and the listing gives each rescue step a section of its own. Its tech spec is [0098](0098-rescue-step-adapters-and-providers.md). Proposal [0106](0106-retry-budget-alone.md) amends it too: a rescue step's outcome and the causes of a rescue have no retriable failure, and its tech spec covers that.

## API

- **`RescueStep`** and, behind the `async` feature, **`AsyncRescueStep`**: a rescue step, whose `run` returns a `RescueOutcome`. A closure returning `Result<RescueOutcome, Error>` is a rescue step that needs nothing.
- **`RescueOutcome`**: `success()` and `failure(reason)`. It has no skip.
- **`RescueFactory`** and **`AsyncRescueFactory`**: how a workflow builds a rescue step for its attempt. `needs()` returns a `RescueNeeds`, and `build()` takes a `RescueResolved`.
- **`RescueNeeds`** declares, in order, inputs and optional inputs (`input`), from the input adapter attached to the rescue step's name or else the data bag, data the failed attempt contributed (`from_rescued_step`), each with an `Input` or an `OptionalInput`, the failure's reason (`reason`, `optional_reason`), the error (`error`, `optional_error`), a contributor and a step reporter. A request for the reason or the error declared again replaces the earlier one.
- **`RescueResolved`** takes what was resolved: the same methods, with the reason and the error handed over by value, so a rescue step holds an `Error` without a lifetime.
- **`StepDescriptor::rescue(factory)`** gives a step of a synchronous workflow its rescue step, and **`StepDescriptor::rescue_async(factory)`** a step of an asynchronous workflow, as steps of each mode are given with `new` and `new_async`. **`StepDescriptor::rescued_by::<R>()`** takes a type implementing **`DescribedRescueStep`**, which the macros generate. None of them takes a name.
- **Events**: `EventBody::StepRescuing { step, cause, rescue }`, with `RescueCause` (`failure`, `abnormal termination`), and `EventBody::StepRescued { step, rescue }`. Both are decisions.
- **Aborts**: a required request for the reason or the error without a value names who made it with `journey::FailureRequester` in the result and `event::FailureRequester` in the event: a step hook, or the rescue step.
- **The listing**: `ListedStep::rescue`, the step's rescue section, whose `step` is the name of its rescue step; tech spec 0098 describes the rest of the section.
- **The violation** `Violation::RescuedStepRetried { step }`, of the kind `ViolationKind::RescuedStepRetried`, displayed `rescued step retried`.
- **`FromRescueRequests`**: a rescue step type built from its requests alone, which `#[derive(RescueStep)]` implements.

## Macros

- **`#[derive(RescueStep)]`** on a struct with named fields, or none: each field is an `#[input]` or `#[from_rescued_step]`, keyed by the field's name unless written, as in `#[from_rescued_step("account")]`, or, with no mark, a `Reason` or an `Error`, recognised by its type. An `Option` field is an optional request; `required` on an `Option` field keys a value that is itself an `Option`.
- **`#[rescue_step]`** on the rescue step's own `impl` block, holding `run(self, ...)`, which takes `&mut Contributor` and a reporter as a step's `run` does, recognised by their types, and `#[service]`, as tech spec 0095 describes, and returns `Result<RescueOutcome, Error>`; `async fn run` makes it asynchronous. The block may write `Self::succeed()` and `Self::fail(reason)`. The attribute takes no arguments, since a rescue step has no name of its own.
- **`rescue = R`** on `#[with_step(...)]` gives the step the rescue step `R`, through `rescued_by::<R>()`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A step has at most one rescue step, and only how to build it | `StepDescriptor` holds `Option<Box<StepDescriptor>>`; giving a rescue step replaces the one it had |
| The rescue step's name is the step's name followed by `_rescue` | `StepName::rescue`, called by every way of giving a rescue step, none of which takes a name. Step names are `&'static str`, so the name is made once per step name for the whole program and kept, and a workflow declared many times does not grow memory |
| A declared step with a rescue step's name is `duplicate step name` | admission counts every rescue step's name with the declared steps' names; `#[workflow]` refuses it at compile time |
| A step with a rescue step and a retry budget is `rescued step retried` | admission; the specification does not let a language make it impossible, so `build()` refuses it. `#[with_step]` refuses `rescue` with a `retry_budget` of 1 or more at compile time |
| `retry on abnormal termination`, on or off, does not prevent the rescue of attempt 1 | the decision logic rescues every failure of attempt 1 before it reads the setting; after the rescue, the step has a retry budget of 0, so a failure or an abnormal termination gives it up with that cause, whatever the setting |
| The rescue step is attempted when attempt 1 fails, after `on step abnormal termination`, and neither `on step retry` nor `on step failure` is called for it | the pure decision logic returns `Rescue` for attempt 1 of a step with a rescue step; the engine calls `on step abnormal termination` before deciding, and calls the retry and failure hooks only for `Retry` and `GiveUp` |
| A step is rescued at most once | a rescued step has a retry budget of 0, so attempt 2 is its last, and only attempt 1 is rescued |
| The rescue step runs as attempt 1 under its own name, with no policies and no retry budget, and its success, failure and abnormal termination follow the rules of any step | the engine runs it through the same attempt as any step. Its descriptor is a step descriptor, whose type could hold policies, a budget and a rescue step of its own, but only the crate builds it, always without them: no public method reaches it |
| Its inputs are resolved as a step's | the engine builds it through the same attempt as any step; tech spec 0098 covers the input adapter attached to its name |
| It may request what a step hook may: the failed attempt's contributions, its reason and its error, each required or optional, in the order declared | `RescueNeeds` declares them, resolved in order; a required one without a value aborts with `required data missing` naming the rescue step; a missing optional contribution emits `optional_input_absent` |
| Only a rescue step can request the failed attempt's data | the requests exist only on `RescueNeeds` and `RescueResolved` |
| The failed attempt's contributions are never committed, but are kept for the rescue step | the engine copies them, uncommitted, into what the rescue step may request; an abnormal termination carries none |
| On success, its contributions are committed, then `step_rescued` is recorded, and the step runs again as attempt 2 | the engine, after the rescue step's attempt |
| On failure, the journey fails naming the rescue step, and the rescued step stays `rescuing`, unless the journey is aborted afterwards | the rescue step is given up under the ordinary rules, and the engine never returns to the rescued step |
| An abort while the rescue step is built or runs names the rescue step | the engine sets the step being run to the rescue step for its attempt, and back to the rescued step for `step_rescued` |
| A rescue step must not skip | types: `RescueOutcome` has no skip, and `DescribedRescueStep` gives no `skip()`; see the excluded scenarios. So `rescue skipped` is not an abort reason in Rust |
| A rescue step is never named by its author | neither the builder nor the macros take a name for it: `#[rescue_step]` refuses arguments |

## Tests

- Unit tests in `itinera-core/src/engine/decision.rs`: `a_step_with_a_rescue_step_is_rescued_after_its_first_attempt_however_it_failed` and `a_rescued_step_is_given_up_after_the_attempt_that_follows_its_rescue`.
- Unit tests in `itinera-core/src/engine/rescue/tests.rs`: `a_failed_first_attempt_is_rescued_then_the_step_is_attempted_again`, `a_step_that_fails_after_its_rescue_is_given_up_without_a_second_rescue`, `a_rescue_step_that_does_not_succeed_fails_the_journey_and_the_step_is_not_attempted_again`, `the_rescue_step_receives_what_the_failed_attempt_left_and_its_contributions_are_committed`, `a_required_reason_an_abnormal_termination_did_not_leave_aborts_naming_the_rescue_step`, `a_reporter_failing_on_a_rescue_decision_aborts_the_journey_naming_the_rescued_step` and, behind `async`, `an_asynchronous_rescue_step_is_awaited_before_the_step_is_attempted_again`.
- Unit test in `itinera-core/src/engine/retry/tests.rs`: `the_rescued_steps_retry_and_failure_hooks_are_not_called_for_the_attempt_it_rescues`.
- Unit test in `itinera-core/src/engine/outcome/tests.rs`: `fail_workflow_from_on_step_abnormal_termination_gives_the_step_up_instead_of_rescuing_it`.
- Unit tests in `itinera-core/src/step/name.rs`: `a_rescue_steps_name_is_the_steps_name_followed_by_rescue` and `a_rescue_steps_name_is_made_once_for_each_step_name`.
- Admission's cases `a_step_named_as_another_steps_rescue_step` and `a_step_with_a_rescue_step_and_a_retry_budget`, in `itinera-core/src/workflow/violation/admission.rs`, and the listing's test, which lists a rescue step.
- The runner's unit tests: the cases `whose_rescue_step_failed`, `whose_rescue_step_failed_before_a_reporter_aborted_the_journey` and `whose_rescue_step_could_not_be_built` of `a_steps_status_and_attempts_are_read_from_its_journeys_events` in `sentences/outcomes/step_status.rs`, and `a_rescue_step_that_skips_is_a_case_error` in `step.rs`.
- Equivalence tests in `crates/itinera/tests/macros/rescue.rs` and, behind `async`, `async_rescue.rs`: a rescue step the macros describe lists and runs as one written for the builder, for each way the first attempt ends, including required requests that abort.
- Compile-fail tests in `crates/itinera/tests/ui.rs`, module `rescue`: a rescue step never skips, every field of a derived rescue step is a request, the reason and the error take no key, the block holds `run`, a rescue step is never named, generic rescue steps are refused, and a rescue step belongs only to a workflow of its own mode; module `workflow`: a step with a rescue step has no retry budget, and no step is named like a rescue step.
- The `@macros` feature `payment.feature` of the runner, whose `ship` step is rescued by a rescue step that packs a fragile order.

## Excluded scenarios

| Scenario, in `failing-rescue.feature` | Tag | Proof in `crates/itinera/tests/proofs.rs` |
|---|---|---|
| "A rescue step that skips aborts the journey" | `@rescue-skip` | `a_rescue_step_cannot_skip`: a rescue step returning `RescueOutcome::skipped()` does not compile, since `RescueOutcome` has no skip; the twin succeeds |

## Done when

Every scenario tagged `@proposal-0003` passes or is proven impossible to express, and 3 is listed in `conformance.json`.
