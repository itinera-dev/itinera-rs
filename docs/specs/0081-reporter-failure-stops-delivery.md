# 0081: A reporter that fails stops the delivery of the event it failed on

Tech spec for [proposal 0081](https://github.com/itinera-dev/spec/blob/main/proposals/0081-reporter-failure-stops-delivery.md), implemented in [#39](https://github.com/itinera-dev/itinera-rs/issues/39). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 makes the default dispatcher follow the rule, and stage 3 adds the executor's reporter wrappers.

## API

- The documentation of `Reporter` and `AsyncReporter` says a reporter should handle its own trouble and return `Ok`, and that the order of reporters matters.
- `DefaultDispatcher` stops at the first reporter that fails and returns its error. On `journey_aborted` it delivers to every reporter, and returns the first error, which the executor ignores.
- The executors wrap each reporter they add to the journey's dispatcher. The wrappers share the journey's state: once one reporter has failed, the others receive only `journey_aborted`, and the one that failed receives nothing more. The journey's result holds the reporter's own error; the dispatcher gets an error with the same message, so that it stops delivering the event.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A reporter that fails aborts the journey with `reporter failed`, whatever the dispatcher | the executor's wrappers, which record the failure even if the dispatcher ignores the error |
| The event a reporter failed on does not reach the reporters after it | `DefaultDispatcher`; for any dispatcher, the executor's wrappers |
| `journey_aborted` reaches every reporter except the one that failed | the executor's wrappers, which share the journey's state |
| A failure while `journey_aborted` is delivered is ignored, and delivery continues | the executor's wrappers, which never pass that error to the dispatcher; `DefaultDispatcher` also delivers past it; and the executor ignores an error the dispatcher returns |

## Tests

- Unit tests in `itinera-core/src/report.rs`: delivery of an event stops at the reporter that failed, and a failure while `journey_aborted` is delivered does not stop its delivery, for both the synchronous and the asynchronous dispatcher.
- Unit tests in `itinera-core/src/executor.rs`: a reporter that fails aborts the journey and only `journey_aborted` reaches the others; it does so whatever the dispatcher, even one that ignores errors; a reporter that fails on the last decision still aborts the journey; a failure while `journey_aborted` is delivered is ignored; a dispatcher that fails while dispatching aborts the journey, and one that fails while `journey_aborted` is delivered is ignored; `journey_aborted` reaches every reporter but the one that failed, even through a dispatcher that stops at the first error; `journey_aborted` names the step during which a reporter failed. In `executor/asynchronous.rs`: an asynchronous reporter that fails aborts the journey, and only `journey_aborted` reaches the others.

## Done when

Every scenario tagged `@proposal-0081` passes, and 81 is listed in `conformance.json`. Planned for stage 7, with 0011.
