# 0122: Custom code that returns an unexpected result

Tech spec for [proposal 0122](https://github.com/itinera-dev/spec/blob/main/proposals/0122-unexpected-results.md), implemented in [#186](https://github.com/itinera-dev/itinera-rs/issues/186). It amends [0054](0054-how-custom-code-fails.md), [0010](0010-hooks-lifecycles-and-roles.md) and [0009](0009-running-a-workflow.md), and, through 0009, [0061](0061-instance-carries-journey-id-and-reporters.md). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done.

## API

- **`itinera::journey::JourneyId`** is a string that is not empty. It is made with `JourneyId::try_from`, from a `&str` or a `String`, or with `JourneyId::try_new`, from an `Arc<str>`, each of which refuses an empty string with `itinera::journey::EmptyJourneyId`; it no longer converts from any string with `From`. So no empty journey ID can exist. The type is declared with the `nutype` crate, which runs the check in `try_new` and derives the rest, around an `Arc<str>`, so that a clone shares its text. `nutype` stays at 0.6, the last release that builds on Rust 1.85.
- **`itinera::instance::InstanceError::EmptyJourneyId`** is what `create()` returns when the workflow's ID generator produces an empty string. The generator still returns `Result<String, Error>`. There is no journey and no event.
- **No other public item changes.** Every call into custom code already has the shape the proposal asks for, and none can return an unexpected result, so `AbortReason` gains no `unexpected result`: no journey could ever be aborted with it, and chapter 3, 3.8 point 1 says the rules about it do not apply to a language that rules it out.

## The shape of every call

Chapter 5, 5.10 point 6 asks for the shape of every call into custom code, and how Rust expresses it. Rust has no null-like value, and the compiler checks the type of what every call returns, futures included.

| Call | Shape | What it returns | Rust's form of nothing |
|---|---|---|---|
| A step's run, synchronous or asynchronous | with a result | `Result<Outcome, Error>`, or a future of it | none: an `Outcome` is a success, a failure or a skip |
| A rescue step's run | with a result | `Result<RescueOutcome, Error>`, or a future of it | none |
| A step hook | with a result | `Result<OnSuccess, Error>` or `Result<OnFailure, Error>`, or a future of it | `KeepDefault` |
| A workflow hook | void | `Result<(), Error>`, or a future of it | void |
| The constructor of a step or a rescue step | with a result | `Result<F::Step<'a>, Error>` from a `StepFactory`, `AsyncStepFactory`, `RescueFactory` or `AsyncRescueFactory`, or the step a closure is | none |
| The constructor of a policy | with a result | the policy, or `Result<P, Error>` from a fallible factory | none |
| The dispatcher factory | with a result | `Result<Self::Dispatcher, Error>`, or a future of it | none |
| An input adapter | with a result | `Result<Supplied, Error>` | `Supplied::FromDataBag` |
| A service provider | with a result | `Result<Provided<T>, Error>`, or a future of it | none: a provider returning nothing is impossible to express, proven for `@no-instance` |
| A reporter, and a dispatcher adding reporters or delivering an event | void | `Result<(), Error>`, or a future of it | void |
| A role operation | void for the executor | the application's own trait: a hook calls it, and the executor never sees what it returns | void |
| A value conversion | none: Rust calls none, and does not claim `value-conversions` | | |

`Result<(), Error>` is how Rust writes a call that returns nothing but may fail, not a type that stands for nothing, such as an empty `Option`, so the void calls are void in Rust's own terms. Every asynchronous call is awaited, and the future it returns has the output type of the synchronous call, so it can only resolve to what its call can return.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A void call's result is never read | types: a void call returns `Result<(), Error>`, so there is nothing to read but whether it failed |
| A call with a result expresses nothing by one explicit form, which is never a null-like value | types: `KeepDefault` and `Supplied::FromDataBag`; Rust has no null-like value |
| A call with a result returning an unexpected result aborts the journey, or refuses `run` | impossible to express: each call's return type admits only the results the specification defines |
| The value null an input adapter returns is checked against the input's type | an input adapter supplies any value, and the input reads it by its exact type, as for any other value: the unit `()`, which serializes as null, is `wrong type` for an `i64` input |
| An asynchronous call, a void one included, is awaited, and its rejection is a throw | the engine awaits every future, and its `Err` is the call's failure |
| A journey ID is a string that is not empty | types: `JourneyId` refuses an empty string when it is made; a generator returns a `String` |
| An ID generator producing anything else makes creating the instance fail, with no journey and no event | `create()` returns `InstanceError::EmptyJourneyId` for an empty string; anything that is not a string does not compile |

## Excluded scenarios

The scenarios tagged `@unexpected-result` and `@non-string-journey-id` are excluded, each proven impossible to express by a test in `crates/itinera/tests/proofs.rs`, with a program breaking the rule that fails to compile, and its twin keeping the rule, which compiles and runs. The broken programs are in `proofs/unexpected-result/` and `proofs/non-string-journey-id/`.

| Scenario | Tag | Proof | Broken program |
|---|---|---|---|
| A step that returns no outcome aborts the journey and is not retried | `@unexpected-result` | `a_step_must_return_an_outcome` | a step returning `Ok(())` |
| A rescue step that returns no outcome aborts the journey | `@unexpected-result` | `a_rescue_step_must_return_an_outcome` | a rescue step returning `Ok(())` |
| A step hook that returns something that is not a lifecycle aborts the journey | `@unexpected-result` | `a_step_hook_cannot_return_something_that_is_not_a_lifecycle` | `on_step_failure` returning a `&str` |
| A step hook that returns a null-like value does not keep the default | `@unexpected-result` | `a_step_hook_must_say_keep_default` | `on_step_failure` returning an `Option<OnFailure>` |
| An input adapter that returns no result aborts the journey | `@unexpected-result` | `an_input_adapter_must_return_what_it_supplies` | an adapter returning an `Option<Supplied>` |
| A service provider that returns a null-like value aborts the journey | `@unexpected-result` | `a_provider_cannot_return_an_optional_instance` | a provider returning an `Option<Provided<_>>` |
| A step whose construction returns no step aborts the journey | `@unexpected-result` | `a_step_factory_must_build_a_step` | a factory whose step is an `Option` of one |
| A step policy whose construction returns no policy aborts the journey | `@unexpected-result` | `a_step_policy_must_build_a_policy` | a step policy built as an `Option` of one |
| A workflow policy whose construction returns no policy refuses run | `@unexpected-result` | `a_workflow_policy_must_build_a_policy` | a workflow policy built as an `Option` of one |
| An asynchronous step that resolves to no outcome aborts the journey | `@unexpected-result` | `an_asynchronous_step_must_resolve_to_an_outcome` | an asynchronous step resolving to `Ok(())` |
| An ID generator that returns something that is not a string fails the instance | `@non-string-journey-id` | `an_id_generator_must_return_a_string` | a generator returning an `i64` |

## Tests

- Unit tests in `itinera-core/src/journey/journey_id.rs`: `a_journey_id_cannot_be_empty` and `a_journey_id_keeps_any_text_that_is_not_empty`.
- Unit tests in `itinera-core/src/instance/builder/tests.rs`: `a_generator_that_produces_an_empty_journey_id_makes_creating_the_instance_fail`.
- The eleven proofs above.
- The conformance runner reads the value null as the unit `()`, tested in `itinera-conformance/src/value.rs`. A workflow hook's sentence "returns a value" scripts it to return nothing, since a Rust workflow hook cannot return a value, as STEPS.md allows; a hook's sentence "is asynchronous" holds the scenario to the asynchronous executor, under which every scripted hook is asynchronous; and "creating the workflow instance failed" reads the `InstanceError` that `create()` returned.

## Done when

Every scenario tagged `@proposal-0122` that Rust can express passes, every one tagged `@unexpected-result` or `@non-string-journey-id` is proven impossible to express, and 122 is listed in `conformance.json`.
