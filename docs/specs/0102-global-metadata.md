# 0102: Global metadata

Tech spec for [proposal 0102](https://github.com/itinera-dev/spec/blob/main/proposals/0102-global-metadata.md), implemented in [#124](https://github.com/itinera-dev/itinera-rs/issues/124). It amends [0008](0008-steps.md), [0010](0010-hooks-lifecycles-and-roles.md), [0011](0011-events.md), [0054](0054-how-custom-code-fails.md), [0061](0061-instance-carries-journey-id-and-reporters.md), [0063](0063-dispatcher-factory.md) and [0095](0095-services-provided-to-steps.md). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done.

## API

- **`itinera::report::GlobalMetadata`** holds entries of a key, a `String`, and a value, an `AnyValue`: one entry per key, in the order the keys were first set, the latest value of each. `insert` sets an entry, `get` reads one, and it iterates over its entries. `GlobalMetadata::JOURNEY_ID` is the key `journey_id`. A reporter that keeps the entries may keep them in one.
- **`Reporter::set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error>`** receives one entry. It does nothing by default, so a reporter that has no use for global metadata needs no code. `AsyncReporter::set_global_metadata` is the same, and synchronous, since receiving an entry is keeping it. It replaces `set_journey_id`.
- **`Dispatcher::set_global_metadata`** and **`AsyncDispatcher::set_global_metadata`** have the same signature, and every dispatcher must write one. `DefaultDispatcher` passes the entry to every reporter, in the order they were added, and stops at the first that fails.
- **`InstanceBuilder::global_metadata<T: Value>(key, value)`** sets an entry of the initial global metadata. `WorkflowInstance::take_global_metadata` gives it to the executor once, and is empty by default for a hand-written instance.
- **`StepReporter::set_global_metadata<T: Value>(key, value)`**, and the same on `AsyncStepReporter`, `HookReporter` and `AsyncHookReporter`, set an entry while the journey runs. A service provider sets one through the `HookReporter` or `AsyncHookReporter` it requests. Each returns `Result<(), Interrupted>`, or a future of it, like an emit call.
- **`EventBody::GlobalMetadataSet { key, source }`** is the fact `global_metadata_set`. Its `source` is an `itinera::event::Source`: the step and attempt, or the hook or service provider, with the step and attempt when there is one.
- **`Abort::ReservedMetadataKey(MetadataSetter)`**, `JourneyAbort::ReservedMetadataKey { source }` and `AbortReason::ReservedMetadataKey` are the abort `reserved metadata key`. The result's `itinera::journey::MetadataSetter` names the step, hook or service provider without the step's name or attempt, since the result names no step; the event's `source` names them.
- **`Refusal::ReservedMetadataKey`** and **`Refusal::GlobalMetadata(Error)`** are the new refusals of `run`.
- **`RecordingReporter::global_metadata()`** gives what the testing reporter received, as a `GlobalMetadata`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Global metadata never enters the data bag or the result, and nothing in the journey can read it | it lives only in the instance, until the executor takes it, and in the reporters; no handle offers a way to read it |
| One entry per key: setting a key again replaces its value, and no entry is removed | `GlobalMetadata::insert`; nothing removes an entry |
| Keys are strings and every value is a value | types: `key: impl Into<String>`, `value: T` with `T: Value`; see the excluded scenarios |
| Only the executor sets `journey_id` | the executor sets it; `run` refuses initial global metadata that sets it with `Refusal::ReservedMetadataKey`, before the dispatcher is created; while the journey runs, the engine aborts with `reserved metadata key` before any reporter receives the entry |
| The abort and the refusal name the reserved key | implied: `journey_id` is the only reserved key, so `Abort::ReservedMetadataKey`, `JourneyAbort::ReservedMetadataKey` and `Refusal::ReservedMetadataKey` carry no key, and their documentation names it |
| A reporter receives each entry; what it keeps is its own choice | `Reporter::set_global_metadata`, which does nothing by default |
| The default dispatcher passes every entry to every reporter, in order | `DefaultDispatcher::set_global_metadata` |
| After adding the reporters and before `journey_started`, the executor sets `journey_id`, then the initial entries, emitting nothing | `engine::introduce`, which both executors call after adding the reporters and before handing the journey to the engine |
| A reporter or the dispatcher that fails while these are set refuses the journey | `introduce` returns the guard's recorded error, or else the dispatcher's, which `run` returns as `Refusal::GlobalMetadata` |
| Steps, hooks and service providers may set entries; input adapters, role operations and constructors cannot | types: only the four reporter handles have `set_global_metadata`, and only steps, hooks and providers can request a reporter |
| The value is captured when it is set | the handle takes `T` by value and erases it into an `AnyValue` |
| The entry reaches the reporters, then `global_metadata_set` is emitted, before the call returns | `Journey::set_global_metadata` in `engine/metadata.rs` delivers through the dispatcher, then emits; the synchronous handles run that to its end, the asynchronous ones return it as a future |
| An entry stands whatever happens afterwards | nothing in the engine withdraws an entry |
| A reporter or the dispatcher that fails on an entry aborts with `reporter failed`, and `journey_aborted` reaches every reporter but that one | the guards: an entry is admitted only while no reporter has failed, and a failure on one fails the journey as on an event; the call returns `Interrupted` |
| A handle used after its call has ended has no effect | types, as for emitting: a handle borrows its attempt or call |

## Tests

- Unit tests in `itinera-core/src/report/global_metadata.rs`: `a_key_inserted_again_keeps_its_place_and_takes_the_new_value`.
- Unit tests in `itinera-core/src/report/default_dispatcher.rs` and `report/asynchronous/dispatcher.rs`: global metadata reaches every reporter, of either kind, in order, until one fails.
- Unit tests in `itinera-core/src/engine/metadata/tests.rs`: the journey ID, then the initial entries, reach the reporters before any event; a step's entry reaches the reporters, then `global_metadata_set` names its key and step; an entry stands though the attempt that set it fails; a step that sets the journey ID aborts the journey and no reporter receives it; a reporter that fails on an entry aborts the journey and interrupts the step; a hook's entry names its policy and hook; an asynchronous step awaits its entry; a dispatcher that fails on an entry a step sets aborts the journey.
- Unit tests in `itinera-core/src/engine/guard/tests.rs`: whatever the dispatcher, a reporter that fails on the journey ID keeps it from the reporters after it, and a reporter that fails on an entry a step sets aborts the journey and only `journey_aborted` reaches the others.
- Unit tests in `itinera-core/src/journey/abort/setter.rs`: the setter names who set the key but not the step.
- Unit tests in `itinera-core/src/executor/tests.rs` and `executor/asynchronous/tests.rs`: initial global metadata that sets the journey ID is refused before the dispatcher is made; a dispatcher, or an asynchronous reporter, that fails on global metadata refuses the journey before any event.
- Unit tests in `itinera-core/src/instance.rs`: an instance hands its initial global metadata over once.

## Excluded scenarios

| Scenario, in `failures.feature` | Tag | Proof in `crates/itinera/tests/proofs.rs` |
|---|---|---|
| "A step's entry that is not a value aborts the journey with "not a value" and reaches no reporter" | `@non-value` | `a_step_cannot_set_a_non_value_as_global_metadata`: setting a closure through the step's reporter does not compile, since the value must be `Value`; the twin sets an integer |
| "Initial global metadata that is not a value is refused by run" | `@non-value` | `initial_global_metadata_cannot_hold_a_non_value`: giving a closure as initial global metadata does not compile; the twin gives an integer |

## Done when

Every scenario tagged `@proposal-0102` passes or is proven impossible to express, and 102 is listed in `conformance.json`.
