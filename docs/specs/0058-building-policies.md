# 0058: Workflow policies are built per journey, step policies per attempt

Tech spec for [proposal 0058](https://github.com/itinera-dev/spec/blob/main/proposals/0058-building-policies.md), implemented in [#26](https://github.com/itinera-dev/itinera-rs/issues/26). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7, which gave policy descriptors their factories and made the executor build policies.

## API

- **A policy descriptor holds the policy's definition**: its name, the hooks it defines with what each needs, and its factory. `StepPolicyDescriptor::new(name, factory)` and `WorkflowPolicyDescriptor::new(name, factory)` take a factory that cannot fail, `impl Fn() -> P`, and `fallible` one that may, `impl Fn() -> Result<P, Error>`; `new_async` and `fallible_async` do the same for an asynchronous workflow, behind the `async` feature. A descriptor never holds an instance of the policy.
- **The executor builds the instances.** `LocalExecutor::run` and `AsyncLocalExecutor::run` build one instance of each workflow policy before the journey starts, before they create its dispatcher; the engine builds one instance of each step policy attached to a step for each of its attempts, right after `attempt_started`.
- **`Refusal::WorkflowPolicy`**, with the policy's name and its error, is the refusal of a workflow policy that cannot be built. **`Abort::PolicyCouldNotBeBuilt`**, with the policy's name and the error, and **`JourneyAbort::PolicyCouldNotBeBuilt`**, with the step, the policy and the error's message, are the abort of a step policy that cannot be built.
- **Hooks take `&self`**: every hook trait's method receives its policy as a shared reference.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A policy's definition is reusable, and its instances are built by the executor, never shared | types: a descriptor holds a factory, not an instance; only the executor calls it |
| One instance of each workflow policy is built before the journey starts, serves the whole journey, and is dropped when it ends | `run` builds them once, in the order attached, and drops them when it returns |
| One new instance of each step policy attached to a step is built for every attempt, right after `attempt_started` and before the step's inputs are resolved; all the hooks of the attempt use it, `on step failure` included, and it is dropped once they return | the engine builds them when the attempt starts and keeps them with the attempt until its hooks have returned |
| A step policy attached to two steps never shares an instance between them | each step's descriptor holds its own policy descriptors, and the engine builds instances from them for that step's attempts only |
| Which hooks a policy defines is fixed by its definition, so `hook defined twice` is caught before any journey | types: the descriptor names the hooks, not the instance; `build()` checks them |
| A workflow policy that fails while it is built is a refusal: no journey starts and no event is emitted | `run` returns `Refusal::WorkflowPolicy` before it creates the dispatcher |
| A step policy that fails while it is built aborts the journey with `policy could not be built`, naming the step and the policy, after `attempt_started` and before any input event; no hook runs afterwards | the engine builds the attempt's policies before resolving the step's inputs, and turns the error into the abort |
| Hooks cannot change their policy, where the language allows it | types: hooks take `&self`, a read-only receiver. A policy holding interior mutability, such as a `Mutex`, can still change itself, but a change lasts only as long as the instance: one journey for a workflow policy, one attempt for a step policy |

## Tests

- Unit tests in `itinera-core/src/engine/hooks.rs`: `step_policies_are_built_for_every_attempt` and `a_step_policy_that_cannot_be_built_aborts_the_journey_before_the_steps_inputs`.
- Unit tests in `itinera-core/src/executor.rs`: `each_journey_gets_new_workflow_policy_instances` and `a_workflow_policy_that_cannot_be_built_refuses_the_journey_before_its_dispatcher`. In `executor/asynchronous.rs`: `an_asynchronous_workflow_policy_that_cannot_be_built_refuses_the_journey_before_any_event`.
- The conformance runner's scripted policies count their own calls and contribute the count, so the cases of 0058 observe how many instances were built, under both executors.

## Done when

Every scenario tagged `@proposal-0058` passes, and 58 is listed in `conformance.json`. Done in stage 7.
