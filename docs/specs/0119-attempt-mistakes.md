# 0119: An attempt scripted twice, an attempt a step can never have, and reading the record of an unknown step

Tech spec for [proposal 0119](https://github.com/itinera-dev/spec/blob/main/proposals/0119-attempt-mistakes.md), implemented in [#178](https://github.com/itinera-dev/itinera-rs/issues/178). It amends [0108](0108-test-support.md). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress, until proposal 119 is listed in `conformance.json`.

## API

No public item is added or removed. What changes is when the script's choices of attempts panic, and what the panic says:

- **`StepScripting::on_attempt(n)`** panics, in this order: for attempt 0, with `charge has no attempt 0; attempts are numbered from 1`; for an attempt past the most the step can have, with `charge has no attempt 3; it can have at most 2`; for an attempt scripted already, by number, by `on_every_attempt()` or by `on_remaining_attempts()`, with `attempt 2 of charge is scripted already`; and for a gap, with `attempt 2 of charge is missing before attempt 3`.
- **`StepScripting::on_every_attempt()`** after any attempt of the step is scripted panics with `attempt 1 of charge is scripted already`, followed by a hint to use `on_remaining_attempts()` when the attempts so far were numbered.
- **`StepScripting::on_remaining_attempts()`** panics with `charge has no attempt 3; it can have at most 2` when the numbered attempts reach that most, and otherwise, after the remaining attempts are scripted already, with `attempt 2 of charge is scripted already`, naming the attempt after the last numbered one.
- **`ScriptedRun::step(name)`** already panicked for a name the workflow does not have, and **`StepRecord::attempt(n)`** already returned `None` for an attempt it does not hold, 0 included.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Choosing an attempt scripted already is a mistake in the test, naming the step and the first attempt chosen again | a panic in `instance/scripted/step.rs` |
| Choosing an attempt the step can never have is a mistake in the test, naming the step and the attempt | a panic in `instance/scripted/step.rs`, against `Target::most_attempts` in `instance/scripted/target.rs` |
| A negative or fractional attempt cannot be chosen | types: `on_attempt` takes a `u32` |
| An attempt the step can never have is named rather than one scripted already or a gap | `on_attempt` and `on_remaining_attempts` check the most attempts first |
| Reading the record of a step the workflow does not have stops the test where it is read | a panic in `journey/scripted_run.rs` |
| Reading an attempt the record does not hold reads nothing | `StepRecord::attempt` returns an `Option` |

## Tests

- Unit tests in `itinera-core/src/instance/scripted/step.rs`: `attempts_are_numbered_from_1`, `an_attempt_cannot_be_scripted_twice`, `attempts_are_scripted_without_gaps`, `a_step_cannot_be_scripted_past_the_attempts_its_retry_budget_allows`, `an_attempt_the_step_cannot_have_is_named_before_a_gap`, `an_attempt_the_step_cannot_have_is_named_before_one_scripted_already`, `an_attempt_scripted_already_is_named_before_a_gap`, `no_attempt_can_be_numbered_after_the_remaining_ones_are_scripted`, `every_attempt_cannot_be_scripted_after_a_numbered_one`, `every_attempt_cannot_be_scripted_after_the_remaining_ones`, `the_remaining_attempts_cannot_be_scripted_twice` and `the_remaining_attempts_cannot_be_scripted_when_none_remain`.
- The runner reads `attempt scripted twice` and `no such attempt` from those panics, and the record's panic for an unknown step, in `testing/mistakes/reported.rs`, tested by `a_mistake_in_choosing_attempts_names_its_step_and_attempt` and the case `unknown_step_in_the_record`. A mistake's row may name attempt 0. The sentences that read the record of a step, stopping the test, and an attempt it does not hold are in `sentences/testing/record.rs`.

## Excluded scenarios

Once proposal 119 is listed, these scenarios in `0108-test-support/script-checks.feature` are excluded, each proven impossible to express by a test in `crates/itinera/tests/proofs.rs` with a program that fails to compile, since `on_attempt` takes a `u32`, and the twin choosing attempt 1, in `attempt-negative/attempt_counted_from_one.rs`, that compiles and runs:

| Scenario | Tag | Proof |
|---|---|---|
| "A script that describes a negative attempt stops the test" | `@attempt-negative` | `a_script_cannot_describe_a_negative_attempt` |
| "A script that describes a fraction of an attempt stops the test" | `@attempt-fraction` | `a_script_cannot_describe_a_fraction_of_an_attempt` |
