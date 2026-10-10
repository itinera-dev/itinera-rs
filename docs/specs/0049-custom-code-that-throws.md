# 0049: Custom code that throws

Tech spec for [proposal 0049](https://github.com/itinera-dev/spec/blob/main/proposals/0049-custom-code-that-throws.md), implemented in [#21](https://github.com/itinera-dev/itinera-rs/issues/21). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals. Proposals 0054, 0058, 0061 and 0081 amend it: in Rust, throwing is returning an `Err`, a panic is never caught, the abort reasons are `hook failed` and `reporter failed`, building a policy joins the calls into custom code, the journey ID generator runs when the instance is created, and the event a reporter failed on stops there.

Status: done in stage 7. Stage 3 added the executors, with their refusals and the reporters' guards; stage 5 the steps' constructors, input adapters and abnormal terminations; stage 7 the hooks, the role operations they call and the policies' factories.

## API

- **Every call into custom code returns `Result<_, itinera::error::Error>`**: the journey ID generator, reporters' `init`, dispatcher factories, dispatchers, reporters, step factories and steps, input adapters, policy factories given to `fallible` or `fallible_async`, hooks, and role operations that can fail, which the hook propagates. A factory that cannot fail is given to `new` or `new_async` instead.
- **`LocalExecutor::run`** and **`AsyncLocalExecutor::run`** return `Result<JourneyResult, Refusal>`: whatever custom code returns inside the journey becomes the result, and never escapes `run`.
- **`itinera::executor::Refusal`** has a variant for each refusal: `WorkflowPolicy`, with the policy's name and its error, `DispatcherFactory` and `Dispatcher`.
- **`itinera::instance::InstanceError`** is what `create()` returns when the journey ID generator or a reporter's `init` fails, before any executor is involved: `JourneyId(Error)` or `Reporter(Error)`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A journey ID generator, custom or default, that fails starts no journey and emits no event, and the caller receives its error | types: `create()` returns `InstanceError::JourneyId`, so there is no instance to run |
| A dispatcher given to the executor that fails while the journey's reporters are added is a refusal, with no event | `run` returns `Refusal::Dispatcher` |
| A workflow policy's factory that fails is a refusal, with no event (proposal 0058) | `run` builds the workflow policies before it creates the dispatcher, and returns `Refusal::WorkflowPolicy` |
| A step policy's factory that fails aborts the journey with `policy could not be built` (proposal 0058) | the engine, right after `attempt_started` |
| A step's factory, or an input adapter for one of its inputs, that fails aborts the journey with `step could not be built` | the engine, after `input_adapter_failed` for an adapter |
| A running step that fails ends its attempt with an abnormal termination, and the journey is not aborted | the engine reports the `Err` with `step_abnormal_termination`, unless a reporter failed while the step ran (proposal 0055) |
| A hook, or a role operation it calls, that fails aborts the journey with `hook failed` | the engine turns the hook's `Err` into the abort; a role operation's error reaches it through the hook's `?` |
| A reporter, or a given dispatcher while dispatching, that fails on any event but `journey_aborted` aborts the journey with `reporter failed` | the executor's wrappers around each reporter, and the executor for the dispatcher's own error |
| A reporter, or a given dispatcher, that fails while `journey_aborted` is delivered is ignored: the first abort reason stands, nothing more is emitted, and delivery goes on to the other reporters | the executor's wrappers never pass that error on, and the executor ignores the dispatcher's |
| Nothing custom code returns escapes `run` | types: `run` returns `Result<JourneyResult, Refusal>`, and every error inside the journey becomes its result |
| A panic is not a throw (proposal 0054) | itinera never catches a panic: it reaches whoever called `run` |

The list is exhaustive: these are the only places where itinera calls code it did not write. The documentation of `Reporter` says that a reporter should handle its own trouble and return `Ok`, since an error aborts the journey.

## Tests

- Unit tests in `itinera-core/src/instance.rs`: `a_generator_that_fails_makes_creating_the_instance_fail` and `a_reporter_that_cannot_be_made_makes_creating_the_instance_fail`.
- Unit tests in `itinera-core/src/executor.rs`: `a_dispatcher_factory_that_fails_refuses_the_journey_before_any_event`; `a_dispatcher_that_fails_while_reporters_are_added_refuses_the_journey_before_any_event`; `a_workflow_policy_that_cannot_be_built_refuses_the_journey_before_its_dispatcher`; `a_reporter_that_fails_aborts_the_journey_and_only_journey_aborted_reaches_the_others`; `a_dispatcher_that_fails_while_dispatching_aborts_the_journey`; `a_reporter_that_fails_while_journey_aborted_is_delivered_is_ignored`; `a_dispatcher_that_fails_while_journey_aborted_is_delivered_is_ignored`. In `executor/asynchronous.rs`, the same refusals and the reporter's abort for asynchronous dispatchers and reporters.
- Unit tests in `itinera-core/src/engine.rs`: `a_step_whose_factory_fails_aborts_the_journey_as_it_could_not_be_built`; `an_input_adapter_that_fails_aborts_the_journey_as_its_step_could_not_be_built`; `an_abnormal_termination_is_retried_only_when_the_step_allows_it`.
- Unit tests in `itinera-core/src/engine/hooks.rs`: `a_hook_that_fails_aborts_the_journey_with_its_error` and `a_step_policy_that_cannot_be_built_aborts_the_journey_before_the_steps_inputs`.
- The conformance scenarios of 0049 run under both executors, with a role operation scripted to fail inside a hook.

## Done when

Every scenario tagged `@proposal-0049` passes, and 49 is listed in `conformance.json`. Done in stage 7.
