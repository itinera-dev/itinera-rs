# 0063: The executor is given a dispatcher factory

Tech spec for [proposal 0063](https://github.com/itinera-dev/spec/blob/main/proposals/0063-dispatcher-factory.md), implemented in [#30](https://github.com/itinera-dev/itinera-rs/issues/30). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 6. Stage 1 added dispatchers and their factories, and stage 3 the executors that use them; stage 6 lists the proposal, once the conformance runner runs workflows.

## API

- **Synchronous:** `Dispatcher`, with `add(Box<dyn Reporter>)` and `dispatch(&Event)`, and `DispatcherFactory`, whose `create(&mut self)` returns its associated `Dispatcher`. `LocalExecutor` takes a `DispatcherFactory`.
- **Asynchronous**, behind the `async` feature: `AsyncDispatcher`, whose `add` takes a `BoxedReporter` (a reporter of either kind), and `AsyncDispatcherFactory`. Their methods return `impl Future + Send`, so they can be written as `async fn`. `AsyncLocalExecutor` takes an `AsyncDispatcherFactory`.
- A synchronous dispatcher accepts only synchronous reporters, so an asynchronous reporter can never reach the synchronous executor.
- `DefaultDispatcherFactory` implements both factory traits. It creates `DefaultDispatcher` for the synchronous executor and `DefaultDispatcher<BoxedReporter>` for the asynchronous one. A sealed trait bounds `DefaultDispatcher`'s parameter, so it holds only those two kinds of reporter.
- Every operation returns `Result<_, Error>`.
- `run` returns `Refusal::DispatcherFactory` when the factory fails, and `Refusal::Dispatcher` when the dispatcher fails while the instance's reporters are added. Both carry the error.

Its API stays behind the `unstable` feature, since it names API of proposals not yet listed, such as the events of 0011 and the policies of 0010, until stage 7 lists them.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A factory or an `add` that fails is a refusal, before any event | the executors |
| One dispatcher per journey, dropped when it ends | the executors: `run` creates it, and it lives only as long as the call |
| The default dispatcher calls reporters in the order they were added | `DefaultDispatcher` |

## Tests

- Unit tests in `itinera-core/src/report.rs`: the default dispatcher calls reporters in the order they were added, and the asynchronous one calls both kinds in order.
- A compile-fail example in the documentation of `DefaultDispatcher`: it holds no other kind of reporter.
- Unit tests in `itinera-core/src/executor.rs`: a dispatcher factory that fails, or a dispatcher that fails while reporters are added, refuses the journey before any event; an executor runs each journey with a new dispatcher. In `executor/asynchronous.rs`: an asynchronous dispatcher factory that fails, or an asynchronous dispatcher that fails while reporters are added, refuses the journey before any event.

## Done when

Every scenario tagged `@proposal-0063` passes, and 63 is listed in `conformance.json`. Done in stage 6.
