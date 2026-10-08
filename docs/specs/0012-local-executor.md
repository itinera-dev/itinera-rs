# 0012: The local executor

Tech spec for [proposal 0012](https://github.com/itinera-dev/spec/blob/main/proposals/0012-local-executor.md), implemented in [#14](https://github.com/itinera-dev/itinera-rs/issues/14). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 3 adds both executors and the engine they share, running a workflow whose one step can only succeed; stage 6 adds the scan and its decisions.

## API

- **`itinera::executor::LocalExecutor<F>`** runs `Synchronous` workflows. `new()` uses the `DefaultDispatcherFactory`; `with_dispatcher_factory(factory)` takes any `DispatcherFactory`. `run(&mut self, instance)` returns `Result<JourneyResult, Refusal>`.
- **`itinera::executor::AsyncLocalExecutor<F>`**, behind the `async` feature, runs workflows of either mode with an `AsyncDispatcherFactory`. Its `run` is an `async fn`, whose future is `Send` whenever the instance is.
- **`itinera::executor::Refusal`** names what refused a journey before it started, with its error: `DispatcherFactory`, or `Dispatcher` while the reporters were added. Stage 7 adds the refusal of a workflow policy that cannot be built.
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
| The asynchronous executor accepts synchronous parts too | types: `AsyncLocalExecutor::run` accepts any mode, and holds every reporter as a `BoxedReporter` |

## Tests

- Unit tests in `itinera-core/src/executor.rs`: a journey emits its events in order to every reporter and succeeds with its data; the step runs once; a journey without a step succeeds; an executor runs journeys one after another, each with a new dispatcher; and the refusal and reporter failure tests listed in the tech specs of 0063 and 0081.
- Unit tests in `itinera-core/src/executor/asynchronous.rs`: an asynchronous workflow emits its events in order to both kinds of reporter, and its future is `Send`; the asynchronous executor runs synchronous workflows.
- Unit tests in `itinera-core/src/engine.rs`: events are numbered from 1, and carry the journey ID, the workflow name and the time of the engine's clock.
- Planned: compile-fail tests for running an instance twice and calling `run` concurrently.

## Done when

Every scenario tagged `@proposal-0012` passes, and 12 is listed in `conformance.json`. Planned for stage 6.
