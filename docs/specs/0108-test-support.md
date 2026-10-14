# 0108: Test support

Tech spec for [proposal 0108](https://github.com/itinera-dev/spec/blob/main/proposals/0108-test-support.md), implemented in [#145](https://github.com/itinera-dev/itinera-rs/issues/145). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals, and the "A scripted attempt" section of the [architecture](../architecture.md) shows how a scripted run goes through the engine.

Status: done.

Test support was built before the proposal was accepted: the recording reporter in [#120](https://github.com/itinera-dev/itinera-rs/pull/120), the recorders in [#128](https://github.com/itinera-dev/itinera-rs/pull/128), scripted runs for [#129](https://github.com/itinera-dev/itinera-rs/issues/129), and the path and fake providers for [#138](https://github.com/itinera-dev/itinera-rs/issues/138). This tech spec maps the proposal's rules to them, and the runner now checks them through the cases tagged `@capability-testing`.

## API

Everything below is behind the `testing` feature of `itinera`, which is off by default, so an application enables it for its tests only, for example as a dev-dependency with `features = ["testing"]`. It is outside `unstable`.

- **The recording reporter** is `itinera::report::RecordingReporter`. Its clones share what they keep, so a test adds a clone to an instance with `reporter(...)` and reads back `events()`, `global_metadata()`, the latest value of each entry, and `path()`, the journey's path as a `Vec<String>`. The lines are written in `report/recording_reporter/path.rs`.
- **Fake providers** are given with `fake_provider::<T>(name, fake)`, on `InstanceBuilder` and on the builder `#[workflow]` generates. `fake` is a function returning `Result<Provided<T>, Error>`, as a provider's does. The test names `T`, such as `dyn PaymentGateway`, because a provider is named by text.
- **Scripted runs** start with `instance.scripted()`, which gives `instance::Scripted`. `.step(name)` and `.and_step(name)` name steps, and rescue steps by their own names. The attempts are chosen with `on_attempt(n)`, `on_every_attempt()` and `on_remaining_attempts()`, then described with `will_contribute`, `will_emit`, `will_emit_with` and `will_set_global_metadata`, in that order, and ended with `will_succeed()`, `will_fail(reason)`, `will_skip()`, `will_skip_because(reason)` or `will_terminate_abnormally(message)`. `will_run_for_real()` comes straight after the attempts are chosen. `ensure_all_scripted()` asks for every step. `run()`, awaited for an `Asynchronous` workflow, returns a `journey::ScriptedRun`, whose `result` is the `JourneyResult` and whose `step(name)` gives a `StepRecord` with `attempts()` and `attempt(n)`. Each `AttemptRecord` gives its `number()` and reads with `input::<T>(key)` what its step was built with, as a `Read<T>`. The ending is public as `step::Ending`, with `outcome()`.
- **Recorders** are `journey::Contributions`, which lends a contributor with `contributor()` and is read with `read::<T>(key)` and `keys()`; `step::StepEventReporter` and `policy::HookEventReporter`, which lend `reporter()`, or `async_reporter()` with the `async` feature, are made failing with `failing()`, and are read with `emitted()`, giving each `event::Emitted` with its `level()`, `message()` and `data::<T>()`, and `global_metadata()`; and `DataBag::default().with(key, value)`, which lends its read access with `access()`.

## Names

Rust spells each suggested name in snake case, and keeps every one. Three names are added:

- `async_reporter()` lends the asynchronous reporter, since Rust has one reporter type per execution mode;
- `emitted()` reads what an event reporter kept, since the suggested names leave the reading of events unnamed;
- `AttemptRecord::number()` gives the attempt's number, so that a test checks which attempts the record holds, not only how many.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A language that claims `testing` offers all five parts and passes the cases | `"testing"` in `capabilities` and 108 in `proposals` of `conformance.json`, so the `run-conformance` action runs every scenario tagged `@capability-testing` |
| Test support only where the application asks for it | the `testing` feature, off by default; `itinera` enables it for its own tests through its dev-dependency on `itinera-core` |
| Nothing changes how a journey runs | a scripted run starts through the same crate-private start as `run` and changes only the engine's runner; a fake is kept in the instance's own copy of the descriptor under the provider's name, keeping its scope, attachment and needs |
| A mistake in a test stops it before the journey, naming each problem | a panic: where the script is written, for a mistake in writing it, in `instance/scripted.rs`, `instance/scripted/step.rs`, `instance/scripted/attempt.rs` and `instance/scripted/target.rs`; in `run()`, listing everything the script leaves out, in `instance/scripted/checks.rs`; and where the fake is given, or at `create()` for the builder `#[workflow]` generates, in `workflow/fake.rs` and `service/fake.rs`. A panic is not a refusal and emits no event |
| The path lists only the flow, in the proposal's words | `report/recording_reporter/path.rs` writes a line for each of the proposal's events and none for any other |
| Every scripted step is still built for real | the script's runner in `engine/script.rs` has the factory build the step and drops it before playing the script |
| A scripted ending has the consequences of the outcome it stands for | the script's runner returns the ending's outcome, or its error, to the engine, which decides as for a real step |
| A contribution, emit or entry of global metadata for a step that declares no contributor or reporter is a mistake | the script checks the step's declared needs where it is written |
| A rescue step cannot be made to skip | `will_skip` and `will_skip_because` panic on a rescue step |
| The record holds every attempt handed to its step's construction | the script's runner records the attempt before building the step; an attempt that ends earlier never reaches it |
| A lent handle behaves as in a journey | a recorder's reporter emits through the same `Reporting` the engine's reporters use, so `failing()` and setting `journey_id` interrupt as in a journey |

## Tests

- Unit tests next to each part in `itinera-core`: `report/recording_reporter.rs` and `report/recording_reporter/path.rs`, `instance/scripted.rs`, `instance/scripted/checks.rs` and `instance/scripted/attempt.rs`, `engine/script/tests.rs`, `journey/scripted_run.rs`, `service/fake/tests.rs`, and the recorders' own modules.
- The runner reaches each part only through `itinera`'s test support, never through its own recording or scripting:
  - the recording reporter is added after the reporters the workflow lists, and a fake in place of each provider a scenario names, in `journey/instance.rs`, with the fakes in `service/fake.rs`;
  - a scripted run writes the script row by row with the builder in `testing/script.rs` and runs it on the local executor of the workflow's mode, with the runner's own recorder as the instance's first reporter, in `journey/scripted.rs`;
  - a step, a hook or an input adapter runs alone with handles the recorders lend, in `step/alone.rs`, `policy/alone.rs` and `answer_alone` in `declaration/input_adapter.rs`, recording which calls to a lent reporter were interrupted;
  - a mistake in a test is caught as a panic, and its message is read back into the cases' names of mistakes, in `testing.rs` and `testing/mistakes.rs`.
- The sentences are in `sentences/testing/`. Unit tests in `itinera-conformance/src/testing.rs`, `testing/mistakes.rs` and `model/testing.rs`.

## Excluded scenarios

None. The scenarios tagged `@undeclared-handles` and `@fake-service-type` describe what Rust can express, a step that declares neither handle and a fake of another service type, so they run.

## Done when

Every scenario tagged `@proposal-0108` passes, and 108 and `testing` are listed in `conformance.json`.
