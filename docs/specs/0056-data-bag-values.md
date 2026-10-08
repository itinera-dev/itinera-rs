# 0056: Everything in the data bag is a value

Tech spec for [proposal 0056](https://github.com/itinera-dev/spec/blob/main/proposals/0056-data-bag-values.md), implemented in [#24](https://github.com/itinera-dev/itinera-rs/issues/24). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 adds values and the type-erased value, stage 3 the data bag holding the initial data, and stage 5 typed reads and contributions.

## API

- **`itinera::value::Value`** is implemented for every `T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static`. Closures, function pointers and handles are not values.
- **`itinera::value::AnyValue`** holds a value whose type has been erased. It can be cloned and serialized, and read back with `downcast_ref::<T>()`, which reads nothing unless `T` is the exact type.
- **`itinera::journey::DataBag`** maps string keys to `AnyValue`s. Only itinera fills it.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Everything in the data bag is a value | types: only `T: Value` can become an `AnyValue` |
| Nothing is serialized in tier 1 | itinera never calls a serializer; `AnyValue` only lets reporters serialize |

## Tests

- Unit tests in `itinera-core/src/value.rs`: a clone of an erased value is independent of the original, an erased value serializes as the value itself, and it is read only as its exact type.

## Excluded scenarios

Every scenario of this proposal is tagged `@non-value`, and is proven impossible to express in stage 5.

## Done when

Every scenario tagged `@proposal-0056` is proven impossible to express, and 56 is listed in `conformance.json`. Planned for stage 5.
