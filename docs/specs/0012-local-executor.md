# 0012: The local executor

Tech spec for [proposal 0012](https://github.com/itinera-dev/spec/blob/main/proposals/0012-local-executor.md), implemented in [#14](https://github.com/itinera-dev/itinera-rs/issues/14). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 6. Stage 3 added both executors and the engine they share, running a workflow whose one step can only succeed. Stage 5 built each step for its attempt and acted on every outcome, with each attempt the step's last; stage 6 added the scan and its decisions.

## API

- **`itinera::executor::LocalExecutor<F>`** runs `Synchronous` workflows. `new()` uses the `DefaultDispatcherFactory`; `with_dispatcher_factory(factory)` takes any `DispatcherFactory`. `run(&mut self, instance)` returns `Result<JourneyResult, Refusal>`.
- **`itinera::executor::AsyncLocalExecutor<F>`**, behind the `async` feature, runs `Asynchronous` workflows with an `AsyncDispatcherFactory`. Its `run` is an `async fn`, whose future is `Send` whenever the instance is.
- **`itinera::executor::Refusal`** names what refused a journey before it started, with its error: `WorkflowPolicy`, with the policy's name, when a workflow policy could not be built, which stage 7 added; `DispatcherFactory`; or `Dispatcher` while the reporters were added.
- **The engine** is private and asynchronous. The synchronous executor polls it with `Waker::noop()`, since a synchronous workflow never waits. It holds no reference to the instance across an await.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| `run` runs exactly one journey and returns its result, without throwing for anything inside the journey | types: `run` returns `Result<JourneyResult, Refusal>`, and errors inside the journey become the result |
| An instance runs at most one journey | types: `run` takes the instance by value |
| One journey at a time per executor | types: `run` takes `&mut self` |
| Nothing of a journey remains in the executor when `run` returns | the executor holds only its factory; the dispatcher and reporters are dropped with the journey |
| One dispatcher per journey | `run` calls the factory once per journey |
| Execution modes: the synchronous executor accepts only synchronous workflows | types: `LocalExecutor::run` accepts only `WorkflowInstance<Mode = Synchronous>` |
| The asynchronous executor accepts only asynchronous workflows, whose reporters may be synchronous | types: `AsyncLocalExecutor::run` accepts only `WorkflowInstance<Mode = Asynchronous>`, whose reporters are each a `BoxedReporter` |

## Tests

- Unit tests in `itinera-core/src/executor.rs`: a journey emits its events in order to every reporter and succeeds with its data; the step runs once; a journey without a step succeeds; an executor runs journeys one after another, each with a new dispatcher; and the refusal and reporter failure tests listed in the tech specs of 0058, 0063 and 0081.
- Unit tests in `itinera-core/src/executor/asynchronous.rs`: an asynchronous workflow emits its events in order to both kinds of reporter, and its future is `Send`; an asynchronous workflow may list only synchronous reporters.
- Unit tests in `itinera-core/src/engine.rs`: events are numbered from 1, and carry the journey ID, the workflow name and the time of the engine's clock.
- Compile-fail tests in `itinera/tests/ui.rs`, each with a twin that compiles and runs: `an_instance_runs_at_most_one_journey`, where the twin runs a new instance instead, and `an_executor_runs_one_journey_at_a_time`, where two scoped threads call `run` on one executor, and the twin's second journey runs after the first thread.
- A proof in `itinera/tests/proofs.rs`, `a_synchronous_executor_cannot_run_an_asynchronous_step`: handing `LocalExecutor` an instance of an asynchronous workflow fails to compile, and its twin, which hands it to `AsyncLocalExecutor`, compiles and runs. It covers the scenario tagged `@mode-not-accepted`, as the tech spec of 0042 says.
- The conformance runner runs the scenarios of 0012 under both executors, running two journeys with one executor where a scenario says so.

## Excluded scenarios

The scenario "A synchronous-only executor refuses an asynchronous step before the journey starts" is tagged `@mode-not-accepted`, and proven impossible to express by `a_synchronous_executor_cannot_run_an_asynchronous_step` in `crates/itinera/tests/proofs.rs`. It hands `LocalExecutor` an instance of an asynchronous workflow, which does not compile, and its twin hands the instance to `AsyncLocalExecutor`, which compiles and runs the journey.

## Done when

Every scenario tagged `@proposal-0012` passes or is proven impossible to express, and 12 is listed in `conformance.json`. Done in stage 6.
