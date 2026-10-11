# 0027: Data a step or hook receives is read-only

Tech spec for [proposal 0027](https://github.com/itinera-dev/spec/blob/main/proposals/0027-received-data-read-only.md), implemented in [#16](https://github.com/itinera-dev/itinera-rs/issues/16). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 7. Stage 5 gave each step its own copy of its inputs, and captured contributions when they are made. Stage 7 did the same for hooks and input adapters.

## API

- **Everything received is the receiver's own.** `Resolved::input` and `optional_input`, for a step, and `Requested::from_step`, `from_workflow` and their optional forms, for a hook or an input adapter, return a `T` that the receiver owns, cloned from the data bag or from the attempt's contributions. `Requested::reason` returns an owned `Reason`. `Requested::error` returns a shared `&Error`, and `Requested::journey_id` a shared `&JourneyId`, neither of which can be changed through.
- **Contributions are captured when made**: `Contributor::contribute(key, value)` takes the value by ownership.
- **The documentation of `itinera::value::Value`** says that whoever receives a value gets its own clone, and that a type whose `Clone` shares mutable state, such as `Arc<Mutex<_>>`, lets one receiver change what another sees and should not be used as a value.
- **An input adapter's read access to the data bag**, a `DataBagAccess`, offers only `read`, which gives the adapter its own clone of the value in `Read::Present`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Every value a step or hook receives is read-only for it | ownership: each receiver gets its own clone of each value; what it receives by reference, the error and the journey ID, is shared and cannot be changed through |
| Changing a received value never changes the data bag, any contribution, or what another step or hook receives | ownership: a change reaches only the receiver's own clone; an `AnyValue`'s clone is independent of the original |
| A contribution is captured when it is made | ownership: `contribute` takes the value, so the step or hook no longer holds it |
| Writing goes only through a contributor | types: neither steps nor hooks receive the data bag, and an input adapter's `DataBagAccess` offers no method that writes |

Ownership cannot be bypassed in safe Rust, apart from a value whose `Clone` shares mutable state, which the documentation of `Value` warns against. In Rust, the scenarios of this proposal pass trivially, as the proposal allows: a change can only reach the receiver's own copy, so the conformance runner's scripts leave such changes out.

## Tests

- Unit tests in `itinera-core/src/value.rs`: `a_clone_of_an_erased_value_is_independent_of_the_original`.
- Unit tests in `itinera-core/src/journey/contributor.rs`: `a_value_is_captured_when_it_is_contributed`.
- The conformance runner's unit test in `itinera-conformance/src/step.rs`, `a_change_to_a_contributed_value_or_a_received_input_is_left_out_of_the_script`, checks that a scripted change to a contributed value or a received input becomes no action.

## Done when

Every scenario tagged `@proposal-0027` passes, and 27 is listed in `conformance.json`. Done in stage 7.
