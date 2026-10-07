# 0063: The executor is given a dispatcher factory

Tech spec for [proposal 0063](https://github.com/itinera-dev/spec/blob/main/proposals/0063-dispatcher-factory.md), implemented in [#30](https://github.com/itinera-dev/itinera-rs/issues/30). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 adds dispatchers and their factories; the executors that use them arrive in stage 3.

## API

- **Synchronous:** `Dispatcher`, with `add(Box<dyn Reporter>)` and `dispatch(&Event)`, and `DispatcherFactory`, whose `create(&mut self)` returns its associated `Dispatcher`. `LocalExecutor` takes a `DispatcherFactory`.
- **Asynchronous**, behind the `async` feature: `AsyncDispatcher`, whose `add` takes a `BoxedReporter` (a reporter of either kind), and `AsyncDispatcherFactory`. Their methods return `impl Future + Send`, so they can be written as `async fn`. `AsyncLocalExecutor` takes an `AsyncDispatcherFactory`.
- A synchronous dispatcher accepts only synchronous reporters, so an asynchronous reporter can never reach the synchronous executor.
- `DefaultDispatcherFactory` implements both factory traits. It creates `DefaultDispatcher` for the synchronous executor and `DefaultDispatcher<BoxedReporter>` for the asynchronous one. A sealed trait bounds `DefaultDispatcher`'s parameter, so it holds only those two kinds of reporter.
- Every operation returns `Result<_, Error>`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A factory or an `add` that fails is a refusal, before any event | the executors, in stage 3 |
| One dispatcher per journey, dropped when it ends | the executors, in stage 3 |
| The default dispatcher calls reporters in the order they were added | `DefaultDispatcher` |

## Tests

- Unit tests in `itinera-core/src/report.rs`: the default dispatcher calls reporters in the order they were added, and the asynchronous one calls both kinds in order.
- A compile-fail example in the documentation of `DefaultDispatcher`: it holds no other kind of reporter.

## Done when

Every scenario tagged `@proposal-0063` passes, and 63 is listed in `conformance.json`. Planned for stage 6.
