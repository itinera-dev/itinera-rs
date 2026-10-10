# 0032: Configuration errors found while running

Tech spec for [proposal 0032](https://github.com/itinera-dev/spec/blob/main/proposals/0032-configuration-errors.md), implemented in [#17](https://github.com/itinera-dev/itinera-rs/issues/17). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals. Proposal 0042 amends it: errors in how a workflow is put together are caught before any journey, so the abort reason `invalid configuration` does not exist; the tech spec of 0042 covers that part.

Status: done in stage 6. Stage 3 added the aborted result and `journey_aborted`, stage 5 the configuration errors a step can cause while a journey runs, and stage 6 the scan that stops at an abort, partway through the steps.

## API

- **`itinera::journey::Abort`** has one variant per abort reason, with what that reason carries, and `reason()` gives its `AbortReason`. The configuration errors found while running are `StepCouldNotBeBuilt`, `PolicyCouldNotBeBuilt`, `RequiredDataMissing` and `WrongType`. The engine produces `PolicyCouldNotBeBuilt`, and the same errors for hooks' requests, from stage 7, when policies are built and hooks run.
- **`itinera::event::JourneyAbort`** is what `journey_aborted` carries: the abort reason, the step concerned, if any, through `step()`, and the details.
- **`JourneyStatus::Aborted(Abort)`** is the result of an aborted journey, with no data bag.

Its API stays behind the `unstable` feature, since it names API of proposals not yet listed, such as the events of 0011 and the policies of 0010, until stage 7 lists them.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A configuration error found while running aborts the journey | the engine: building a step returns the abort as an error value, which ends the scan |
| The events already emitted remain | the engine only ever appends to the stream, delivering each event as it happens |
| `journey_aborted` is the last event, naming the step, the reason and its details | the engine emits it as the journey's end, and emits nothing after it |
| The result is aborted, with the same reason and details; the step is named by `journey_aborted` alone, since the result carries no step names | each abort is made by one engine function, which builds both the `JourneyAbort` of `journey_aborted` and the result's `Abort` |
| A hook or reporter that fails is a fault, with the same rules for the stream | the engine ends the journey the same way with `reporter failed`; `hook failed` comes with hooks, in stage 7 |

## Tests

- Unit tests in `itinera-core/src/engine.rs`: required data missing, a value of the wrong type and a step that cannot be built each abort the journey with their own abort reason, and `journey_aborted` is the last event.
- The conformance scenario of 0032 tagged with no other proposal runs a three-step workflow whose third step lacks its data, under both executors.

## Done when

Every scenario tagged `@proposal-0032` passes, and 32 is listed in `conformance.json`. Done in stage 6; the scenario also tagged `@proposal-0083` runs once 83 is listed.
