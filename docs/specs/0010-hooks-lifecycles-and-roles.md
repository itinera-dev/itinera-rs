# 0010: Hooks, lifecycles and workflow roles

Tech spec for [proposal 0010](https://github.com/itinera-dev/spec/blob/main/proposals/0010-hooks-lifecycles-and-roles.md), implemented in [#12](https://github.com/itinera-dev/itinera-rs/issues/12). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals. Proposals 0041, 0057, 0058 and 0083 amend it; the tech spec of 0041 covers where hooks' data from the workflow comes from, and that of 0058 when policies are built.

Status: done in stage 7. Stage 4 added policy descriptors naming the hooks they define, and `hook defined twice`. Stage 6 added the hook points around the step decision, where a lifecycle overrides the default. Stage 7 added the policies themselves: one trait per hook, what each hook may request, lifecycles, workflow hooks, hooks' contributions and events, and roles.

## API

- **One trait per hook**, in `itinera::policy`: `OnStepSuccess<W>`, `OnStepFailure<W>`, `OnStepRetry<W>`, `OnStepAbnormalTermination<W>`, `OnWorkflowSuccess<W>` and `OnWorkflowFailure<W>`, for `Synchronous` workflows, and their twins `AsyncOnStepSuccess<W>` and so on, behind the `async` feature, for `Asynchronous` ones. A policy's type implements one for each hook it defines, for the workflows `W` it may serve. Each trait has `fn needs() -> HookNeeds<H>`, which needs nothing unless the policy says otherwise, and the hook itself, which takes `&self` and a `Requested<'_, W, H>` by value.
- **Hook kinds** are the marker types `StepSuccess`, `StepFailure`, `StepRetry`, `StepAbnormalTermination`, `WorkflowSuccess` and `WorkflowFailure`, and `InputAdapter` for input adapters. Sealed traits group them: `HookKind` holds every kind, `StepHookKind` the step hooks, `FailureHookKind` those that may request the reason, `ErrorHookKind` those that may request the error, and `PolicyHookKind` the hooks of policies. `HookNeeds<H>` and `Requested<'_, W, H>` offer a request only to the kinds allowed to make it, so any other request does not compile:

  | Request | Declared with `HookNeeds` | Taken from `Requested` | Kinds |
  |---|---|---|---|
  | data from the workflow | `from_workflow`, `optional_from_workflow` | `from_workflow`, `optional_from_workflow` | every kind |
  | the journey ID | nothing to declare | `journey_id()` | every kind |
  | data from the step | `from_step`, `optional_from_step` | `from_step`, `optional_from_step` | step hooks |
  | the step's name | nothing to declare | `step_name()` | step hooks, input adapters |
  | the attempt number | nothing to declare | `attempt()` | step hooks |
  | the failure's reason | `reason`, `optional_reason` | `reason`, `optional_reason` | `on step failure`, `on step retry` |
  | the error | `error`, `optional_error` | `error`, `optional_error` | `on step failure`, `on step retry`, `on step abnormal termination` |
  | the cause | nothing to declare | `cause()`: a `StepFailureCause` or a `RetryCause` | `on step failure`, `on step retry` |
  | a contributor | `contributor` | `contributor` | hooks of policies |
  | a reporter | `reporter` | `reporter`: a `HookReporter`, or an `AsyncHookReporter` in an asynchronous workflow | hooks of policies |
  | a role | nothing to declare | `role::<dyn R>()` | hooks of policies |

- **Lifecycles are return types.** `on step success` returns `Result<Option<OnSuccess>, Error>`, where `OnSuccess` is `FinishWorkflow` or `FailWorkflow(Reason)`. `on step failure`, `on step retry` and `on step abnormal termination` return `Result<Option<FailWorkflow>, Error>`. Workflow hooks return `Result<(), Error>`. `None` keeps the default. `hook_called` reports what a hook returned as a `Lifecycle`, or `None`.
- **Policy descriptors.** `StepPolicyDescriptor::new(name, factory)`, or `fallible` for a factory that may fail, and `new_async` and `fallible_async` for an asynchronous workflow, return a `Hookless` descriptor. `.on_step_success()` and the other step hooks make it `Hooked`, declaring the hook with what its trait's `needs()` says, and `.on_step_success_needing(needs)` and the like declare it with other needs, for this descriptor only, so that one policy type can read a different key on each step. `WorkflowPolicyDescriptor` does the same with `.on_workflow_success()` and `.on_workflow_failure()`. Only a `Hooked` descriptor can be attached, with `StepDescriptor::policy` or `WorkflowBuilder::policy`.
- **Roles** are plain traits that the workflow's own type implements. A workflow provides a role `R` by implementing `Provides<dyn R>`, whose `role(&self)` returns itself as the role. A hook takes it with `got.role::<dyn R>()`, which compiles only where `W: Provides<dyn R>`, so a policy that uses a role implements its hook only for such workflows, as in `impl<W: Provides<dyn Notifier> + Send + Sync + 'static> OnStepSuccess<W> for Tell`. A role operation that can fail returns `Result<_, itinera::error::Error>`, which the hook propagates.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| For each outcome, the step hooks that run and their order | the engine, around the pure decision function: `on step abnormal termination` first, then `on step retry` while a retry is possible, otherwise `step_given_up` and `on step failure`; no hook for a skip |
| A hook returns a lifecycle it may return, or nothing | types: each hook's return type holds only the lifecycles it may return; see the excluded scenarios |
| `FinishWorkflow` ends the journey at once as succeeded, and the steps left do not run | the engine ends the scan, and `journey_succeeded` names the policy |
| `FailWorkflow` ends the journey at once as failed, and the result names the step and the reason | the engine fails the journey with `Failure::FailWorkflow`, and `journey_failed` names the step, the policy and the hook |
| From `on step success`, the step stays succeeded and its contributions are committed | the engine calls `on step success` after the attempt's contributions are committed |
| From `on step retry`, the step is given up at once, without `on step failure` | the engine gives the step up with `FailWorkflow` as the cause, and calls no other hook |
| Workflow hooks have no lifecycle in tier 1 | types: they return `Result<(), Error>` |
| `on workflow success` runs once when the journey succeeds, `on workflow failure` once when it fails, both before the result; neither runs on an abort | the engine calls the hook for how the journey ended before `journey_succeeded` or `journey_failed`; an abort ends the journey without either |
| A hook that fails aborts the journey with `hook failed`, and no hook runs afterwards | types: every hook returns a `Result`; the engine turns an `Err` into the abort, which ends the journey |
| What a hook may read: data from the step, the reason, the cause and the error for step hooks; the attempt number for step hooks, the journey ID for every hook | types: what `HookNeeds` and `Requested` offer each kind |
| Data from the workflow is requested by key and type, required or optional, and a hook is never given the data bag | the engine resolves it before the hook runs, from the data bag (proposal 0041); types: a policy's `Requested` has no method that gives the data bag |
| Everything a hook receives is read-only for it | the hook receives its own clone of each value and a shared reference to the error, and takes its policy as `&self`; see the tech spec of [0027](0027-received-data-read-only.md) |
| A hook may request a contributor, never the data bag; its contributions are committed once it returns, whatever the step's outcome, and recorded as the hook's | the engine commits them after `hook_called`, and `contribution_committed` carries `Source::Hook` |
| A workflow hook's contributions are part of the journey's output | the engine commits them before `journey_succeeded` or `journey_failed`, so they are in the result's data bag |
| A hook requests each role it needs separately, and receives the workflow only through them | types: `Requested::role` returns the role, and nothing returns the workflow itself |
| A role never exposes the data bag, the executor or the steps | types: a role is a trait of the workflow's own type, which holds none of them |
| Steps never request roles | types: a step's factory and the step never receive the workflow's type |
| A policy attached to a workflow that does not provide a role it requests is refused before anything runs | types: the policy implements its hook only for workflows that provide the role, so naming that hook on a descriptor of another workflow does not compile; see the excluded scenarios |
| The executor does not record role calls | the engine emits nothing for them |
| Hooks are synchronous or asynchronous with their workflow | types: a `Synchronous` workflow's policy descriptors take the synchronous hook traits, and an `Asynchronous` one's the asynchronous traits |

A hook's requests are resolved in the order they were declared, across kinds, and the first required one without a value aborts the journey with `required data missing`. The specification does not say yet in which order requests are resolved; [spec#92](https://github.com/itinera-dev/spec/issues/92) asks it to.

## Excluded scenarios

Two scenarios are proven impossible to express by tests in `crates/itinera/tests/proofs.rs`, each with a program that fails to compile and a twin that compiles and runs:

| Scenario | Tag | Proof |
|---|---|---|
| "A lifecycle a hook may not return aborts the journey", in `order-and-lifecycles.feature` | `@invalid-lifecycle` | `a_failure_hook_cannot_finish_the_workflow` |
| "A policy needing a role the workflow does not provide is refused at admission", in `roles.feature` | `@role-not-provided` | `a_policy_cannot_be_attached_to_a_workflow_that_does_not_provide_its_role` |

- `a_failure_hook_cannot_finish_the_workflow` compiles `tests/proofs/invalid-lifecycle/finish_workflow_on_step_failure.rs`, whose `on step failure` returns `OnSuccess::FinishWorkflow`, which fails, since the hook returns an `Option<FailWorkflow>`. Its twin, `fail_workflow_on_step_failure.rs`, returns a `FailWorkflow` and runs the journey.
- `a_policy_cannot_be_attached_to_a_workflow_that_does_not_provide_its_role` compiles `tests/proofs/role-not-provided/role_not_provided.rs`, whose policy calls `got.role::<dyn Notifier>()` in `on step success` and is attached to a workflow that does not implement `Provides<dyn Notifier>`. Naming the hook fails, with the message that the workflow does not provide the role. Its twin, `role_provided.rs`, differs only in that the workflow provides the role, and builds the workflow.

## Tests

- Unit tests in `itinera-core/src/engine.rs`: `step_hooks_are_called_after_each_attempt_in_order_around_the_step_decision`; `finish_workflow_from_on_step_success_succeeds_the_journey_without_the_steps_left`; `fail_workflow_from_on_step_success_fails_the_journey_after_committing_the_contributions`; `fail_workflow_from_a_hook_before_the_decision_gives_the_step_up_without_on_step_failure`; `fail_workflow_from_on_step_failure_gives_the_journey_its_reason_after_the_step_is_given_up`; with the `async` feature, `an_asynchronous_hook_is_awaited_and_decides_as_a_synchronous_one` and `an_asynchronous_workflow_hook_is_awaited_before_the_journey_is_reported`.
- Unit tests in `itinera-core/src/engine/hooks.rs`: `a_step_hook_receives_what_it_requests_from_the_step_and_the_workflow_before_it_runs`; `a_hook_needs_what_its_attachment_declares`; `a_required_request_without_a_value_aborts_the_journey_before_the_hook_runs`, with a case for data from the step, data from the workflow, the reason of an abnormal termination and the error of a failure; `a_hooks_requests_are_resolved_in_the_order_declared`; `a_reason_declared_again_replaces_the_earlier_declaration`; `a_value_of_another_type_for_a_hook_aborts_the_journey_naming_the_hook`; `a_hooks_contributions_are_committed_once_it_returns_with_the_hook_as_their_source`; `a_hook_that_fails_aborts_the_journey_with_its_error`; `the_workflow_hook_for_how_the_journey_ended_runs_before_it_is_reported`; `a_workflow_hooks_contributions_are_committed_before_the_journey_is_reported`; `no_workflow_hook_runs_when_the_journey_is_aborted`.
- Unit tests in `itinera-core/src/policy.rs`: `hooks_lifecycles_and_causes_display_as_the_specification_writes_them`, and the two tests of a hook named twice on one descriptor.
- Compile-fail tests in `itinera/tests/ui.rs`: `a_policy_attached_defines_at_least_one_hook`, and `a_hook_requests_only_what_its_kind_may_request`, where `on step success` cannot request the failure's reason and `on step failure` can.
- The two proofs above, in `itinera/tests/proofs.rs`.
- The conformance runner's scenario workflow provides its roles through `Provides`, and its scripted policies call them, so a role operation that fails aborts the journey with `hook failed`. The runner's unit tests in `itinera-conformance/src/role.rs` check that a call is recorded before its operation fails, and that calling an operation the workflow does not provide fails without being recorded.

## Done when

Every scenario tagged `@proposal-0010` passes or is proven impossible to express, and 10 is listed in `conformance.json`. Done in stage 7, with the entries for `invalid-lifecycle` and `role-not-provided` under `impossible`.
