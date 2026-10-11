# 0091: Input adapters may read the data bag

Tech spec for [proposal 0091](https://github.com/itinera-dev/spec/blob/main/proposals/0091-adapters-read-the-data-bag.md), implemented in [#85](https://github.com/itinera-dev/itinera-rs/issues/85). It amends [0060](0060-input-adapters-attached-to-steps.md), [0010](0010-hooks-lifecycles-and-roles.md) and [0054](0054-how-custom-code-fails.md). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done.

## API

- **`HookNeeds::<InputAdapter>::data_bag()`** declares read access to the data bag. Only `HookNeeds<InputAdapter>` offers it.
- **`Requested::<'a, W, InputAdapter>::data_bag()`** takes it, as an `itinera::journey::DataBagAccess<'a>`, where `'a` is the adapter's call. It fails when the adapter did not declare it, or took it already, so an adapter that does not declare it does not receive it.
- **`DataBagAccess::read::<T>(key)`** gives an `itinera::journey::Read<T>`: `Present(value)`, a clone of the value, which is the adapter's own copy; `Absent` when the data bag holds nothing under the key; or `OtherType` when it holds a value of another type than `T`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| An adapter receives the access only if it declares it | `Requested::data_bag()` fails unless `HookNeeds::data_bag()` declared it |
| Only input adapters may declare it; hooks of policies are never given the data bag | types: `data_bag()` exists only on `HookNeeds<InputAdapter>` and `Requested<'_, W, InputAdapter>` |
| It is resolved in the order the adapter declares its requests | resolving it cannot fail and emits nothing, so its place in that order is not observable; the engine gives it once the adapter's other requests are resolved |
| It shows everything committed before this attempt, including the earlier attempts' hooks' contributions, never a failed attempt's | the engine gives it the data bag the step is being built from, to which a failed attempt's contributions were never committed |
| A read reports a missing key as absent and a value of another type as such, emits no event and aborts nothing | `DataBagAccess::read` returns a `Read`, and the engine is not involved in it |
| The adapter decides what to return; a value it supplies is checked against the step's type, and a failure aborts with `step could not be built` | unchanged from 0060 |
| Nothing done through the access changes the data bag, and what it reads is its own copy | types: `DataBagAccess` holds a shared reference and offers only `read`, which clones; see the excluded scenarios |
| The access is valid only during the call | types: its lifetime is the call's; see the excluded scenarios |

## Tests

- Unit tests in `itinera-core/src/journey/access.rs`: `a_read_reports_what_the_data_bag_holds`, with the cases `a_value_of_the_type_asked_for` and `a_key_the_data_bag_does_not_hold`, and `a_read_as_another_type_reports_it`.
- Unit tests in `itinera-core/src/engine.rs`: `an_input_adapter_reads_the_data_bag_without_events` and `an_input_adapter_that_did_not_declare_the_data_bag_cannot_read_it`.

## Excluded scenarios

| Scenario, in `data-bag-access.feature` | Tag | Proof in `crates/itinera/tests/proofs.rs` |
|---|---|---|
| "An adapter that writes through its access leaves the data bag unchanged" | `@bag-write` | `an_adapter_cannot_write_through_its_access_to_the_data_bag`: a write through the access does not compile, since it has no method to write; the twin reads through it |
| "An adapter's access kept from an earlier call shows nothing later" | `@late-handle` | `an_adapters_access_to_the_data_bag_cannot_outlive_its_call`: a workflow that keeps the access in a field beyond the call does not compile, nor does an adapter written to take a call that lasts forever, which `InputAdapterDescriptor::new` refuses since it requires an adapter for every call's lifetime; the twin reads through it during the call |

## Done when

Every scenario tagged `@proposal-0091` passes or is proven impossible to express, and 91 is listed in `conformance.json`.
