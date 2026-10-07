# 0081: A reporter that fails stops the delivery of the event it failed on

Tech spec for [proposal 0081](https://github.com/itinera-dev/spec/blob/main/proposals/0081-reporter-failure-stops-delivery.md), implemented in [#39](https://github.com/itinera-dev/itinera-rs/issues/39). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 makes the default dispatcher follow the rule; the executor's reporter wrappers arrive in stage 3.

## API

- The documentation of `Reporter` and `AsyncReporter` says a reporter should handle its own trouble and return `Ok`, and that the order of reporters matters.
- `DefaultDispatcher` stops at the first reporter that fails and returns its error. On `journey_aborted` it delivers to every reporter, and returns the first error, which the executor ignores.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| The event a reporter failed on does not reach the reporters after it | `DefaultDispatcher`; for any dispatcher, the executor's wrappers |
| `journey_aborted` reaches every reporter except the one that failed | the executor's wrappers, in stage 3, which share the journey's state |
| A failure while `journey_aborted` is delivered is ignored, and delivery continues | `DefaultDispatcher`, and the executor ignoring the error |

## Tests

- Unit tests in `itinera-core/src/report.rs`: delivery of an event stops at the reporter that failed, and a failure while `journey_aborted` is delivered does not stop its delivery, for both the synchronous and the asynchronous dispatcher.

## Done when

Every scenario tagged `@proposal-0081` passes, and 81 is listed in `conformance.json`. Planned for stage 7, with 0011.
