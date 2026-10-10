# 0056: Everything in the data bag is a value

Tech spec for [proposal 0056](https://github.com/itinera-dev/spec/blob/main/proposals/0056-data-bag-values.md), implemented in [#24](https://github.com/itinera-dev/itinera-rs/issues/24). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 5. Stage 1 added values and the type-erased value, stage 3 the data bag holding the initial data, and stage 5 typed reads, contributions and the proofs.

## API

- **`itinera::value::Value`** is implemented for every `T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static`. Closures, function pointers and handles are not values.
- **`itinera::value::AnyValue`** holds a value whose type has been erased. It can be cloned and serialized, and read back with `downcast_ref::<T>()`, which reads nothing unless `T` is the exact type.
- **`itinera::journey::DataBag`** maps string keys to `AnyValue`s. Only itinera fills it.
- **`itinera::step::Input<T>`** and **`itinera::step::OptionalInput<T>`** are typed tokens for a step's inputs, where `T: Value`. A step's `needs` lists them, and its factory takes each resolved value with `Resolved::input` or `Resolved::optional_input`, already of type `T`.
- **Input adapters** return an `Option<AnyValue>`, so what they supply is a value too.
- **`itinera::journey::Contributor::contribute`** takes a key and any `T: Value`, by value.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Everything in the data bag is a value: initial data, contributions and what input adapters supply | types: only `T: Value` can become an `AnyValue`, and `data`, `contribute` and input adapters take values only |
| A value is captured when it is contributed | ownership: `contribute` takes the value |
| The same key twice in one attempt: the last value wins | the attempt's contributions keep each key once, with its last value |
| Committed only on success, in order, with `data_overwritten` when a key is replaced | the engine commits an attempt's contributions after `step_succeeded`, through `WorkflowInstance::commit`, which says whether it replaced a value |
| An input, from the data bag or from an input adapter, has the type the step declares, or the journey is aborted with `wrong type` | the engine checks each value's exact type against the token before building the step, so an `i32` is not an `i64`. The abort names the step and the key, and the input adapter when it supplied the value; an adapter that returned nothing is not named |
| Nothing is serialized in tier 1 | itinera never calls a serializer; `AnyValue` only lets reporters serialize |

## Tests

- Unit tests in `itinera-core/src/value.rs`: a clone of an erased value is independent of the original, an erased value serializes as the value itself, and it is read only as its exact type.
- Unit tests in `itinera-core/src/engine.rs`: a step is built with the inputs it declares, read from the data bag or supplied by its input adapter; a value of another type, from either, aborts with `wrong type`, naming the adapter when it supplied the value, and not when it returned nothing.
- Unit tests in `itinera-core/src/step/needs.rs`: an input is taken once, and only as the type it was declared with.
- Unit tests in `itinera-core/src/journey/contributor.rs`: a key contributed twice keeps its last value in its first place; a value is captured when contributed.
- Unit tests in `itinera-core/src/engine.rs`: a successful attempt's contributions are committed in order, reporting what they overwrote; an attempt that does not succeed commits nothing.

## Excluded scenarios

Every scenario of this proposal is tagged `@non-value`, and proven impossible to express by a test in `crates/itinera/tests/proofs.rs`:

| Scenario | Proof |
|---|---|
| A step that contributes a non-value aborts the journey | `a_step_cannot_contribute_a_non_value` |
| Initial data holding a non-value is refused before any journey | `initial_data_cannot_hold_a_non_value` |
| An input adapter that supplies a non-value aborts with "step could not be built" | `an_input_adapter_cannot_supply_a_non_value` |

## Done when

Every scenario tagged `@proposal-0056` is proven impossible to express, and 56 is listed in `conformance.json`. Done in stage 5.
