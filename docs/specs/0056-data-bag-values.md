# 0056: Everything in the data bag is a value

Tech spec for [proposal 0056](https://github.com/itinera-dev/spec/blob/main/proposals/0056-data-bag-values.md), implemented in [#24](https://github.com/itinera-dev/itinera-rs/issues/24). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 adds values and the type-erased value, stage 3 the data bag holding the initial data, and stage 5 typed reads and contributions.

## API

- **`itinera::value::Value`** is implemented for every `T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static`. Closures, function pointers and handles are not values.
- **`itinera::value::AnyValue`** holds a value whose type has been erased. It can be cloned and serialized, and read back with `downcast_ref::<T>()`, which reads nothing unless `T` is the exact type.
- **`itinera::journey::DataBag`** maps string keys to `AnyValue`s. Only itinera fills it.
- **`itinera::step::Input<T>`** and **`itinera::step::OptionalInput<T>`** are typed tokens for a step's inputs, where `T: Value`. A step's `needs` lists them, and its factory takes each resolved value with `Resolved::input` or `Resolved::optional_input`, already of type `T`.
- **Input adapters** return an `Option<AnyValue>`, so what they supply is a value too.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Everything in the data bag is a value | types: only `T: Value` can become an `AnyValue` |
| An input, from the data bag or from an input adapter, has the type the step declares, or the journey is aborted with `wrong type` | the engine checks each value's exact type against the token before building the step, so an `i32` is not an `i64`. The abort names the input adapter when it supplied the value, since the adapter knows the types of the steps it serves, and the step otherwise |
| Nothing is serialized in tier 1 | itinera never calls a serializer; `AnyValue` only lets reporters serialize |

## Tests

- Unit tests in `itinera-core/src/value.rs`: a clone of an erased value is independent of the original, an erased value serializes as the value itself, and it is read only as its exact type.
- Unit tests in `itinera-core/src/engine.rs`: a step is built with the inputs it declares, read from the data bag or supplied by its input adapter; a value of another type, from either, aborts with `wrong type`, naming the adapter when it supplied the value.
- Unit tests in `itinera-core/src/step/needs.rs`: an input is taken once, and only as the type it was declared with.

## Excluded scenarios

Every scenario of this proposal is tagged `@non-value`, and is proven impossible to express in stage 5.

## Done when

Every scenario tagged `@proposal-0056` is proven impossible to express, and 56 is listed in `conformance.json`. Planned for stage 5.
