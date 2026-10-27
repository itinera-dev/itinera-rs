# 0121: Copies that keep received data read-only

Tech spec for [proposal 0121](https://github.com/itinera-dev/spec/blob/main/proposals/0121-read-only-copies.md), implemented in [#185](https://github.com/itinera-dev/itinera-rs/issues/185). It amends [0027](0027-received-data-read-only.md), [0091](0091-adapters-read-the-data-bag.md) and proposal 0113, which Rust does not list. The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done.

## API

No public item is added or removed. The documentation of `itinera::value::Value` says that a value's `Clone` is how each receiver gets its own copy, so it should copy the content as it is, and that a clone that panics stops the journey as any panic does.

## How the rules are enforced

Rust already gave every receiver its own copy, or a view it cannot change through, as the tech spec of [0027](0027-received-data-read-only.md) says. That copy is the value's `Clone`, called through the erased value's `clone_box`. `Clone` is code the application supplies, derived or written by hand, and itinera calls it only to copy a value: it returns `Self`, a value of the same type, and is meant to keep the content. It is the copy proposal 0121 describes, and not a value conversion. itinera calls no other code of the application on a value, neither `Serialize` nor `PartialEq`, `Hash` or `Ord`, so Rust calls no value conversion and still does not claim `value-conversions`.

| Rule | Enforced by |
|---|---|
| A copy is not a value conversion, and calling one does not claim `value-conversions` | `Clone` keeps the type, and is the only application code itinera calls on a value |
| A copy that calls application code is made for each receiver | ownership: the engine clones a value for each request it resolves, and `DataBag::read` for each read, and the receiver owns the clone; `DataBag::get` and `AnyValue::downcast_ref` copy nothing and lend a shared reference, which calls no application code |
| Every receiver can change its own copy without changing the data bag or what another receiver gets | ownership: a change reaches only the receiver's clone, and a shared reference cannot be changed through |
| A copy is never shared between receivers in a way that turns a change into a failure | a clone is owned by its receiver, and what is lent by reference is a read-only view, a change to which does not compile, so no change can fail while the journey runs |
| A copy that fails while a value is read makes it of the wrong type | cannot occur: `Clone::clone` returns `Self`, with no error to return, so it has no recoverable failure |
| A copy that fails while a value is captured makes it not a value | cannot occur: a capture takes the value by ownership, and any clone itinera makes of it later is a `Clone`, which cannot fail |
| A copy whose content differs from its original is not detected | nothing compares a clone with its original |

A clone that panics is an unrecoverable failure, outside the model (chapter 5, 5.10 point 5): itinera does not catch it, and it reaches whoever called `run`.

### Where values are copied

| Where | What happens to the value |
|---|---|
| A step's or a rescue step's input, and data from the step or from the workflow that a hook, an input adapter or a service provider requests | cloned by the engine when it resolves the request, for each request, and moved to the receiver, by `Resolved::input` for a step |
| `Requested::reason` and `optional_reason`, for a failure hook or a rescue step | the reason, its details included, cloned for each hook |
| `DataBag::read`, through an input adapter's access to the data bag | cloned for each read, into `Read::Present` |
| `DataBag::get`, through an input adapter's access to the data bag | not copied: a shared reference, read-only |
| `Contributor::contribute`, event data, a reason's details, global metadata, initial data and initial global metadata, an adapter's `Supplied::value` and a provider's `Provided::with_data` | taken by ownership and erased with `AnyValue::new`, without a copy |
| Later, inside the engine | cloned with `Clone` only: when a contribution is committed to the data bag, when a rescue step gets the failed attempt's contributions, and when a reason is carried to a hook, a failure or the result; none of these can fail |

Test support clones too, with `Clone` alone: the recording reporter keeps a clone of each event, and a scripted run a clone of what each attempt contributed.

Cloning a `HashSet`, a `HashMap` or a `BTreeMap` clones its elements without hashing or comparing them again, and neither `Hash`, `Eq` nor `Ord` can return an error, so a set's or a map's own code adds no failure either.

## Tests

The rows that cannot occur need no test of their own: a `Clone` that returns an error cannot be written. What holds is already tested by the tests the tech spec of [0027](0027-received-data-read-only.md) lists, `a_clone_of_an_erased_value_is_independent_of_the_original` and `a_value_is_captured_when_it_is_contributed`.

## Done when

The proposal adds no scenario. 121 is listed in `conformance.json`, with the cases pinned at `v0.1.0-rc.17`.
