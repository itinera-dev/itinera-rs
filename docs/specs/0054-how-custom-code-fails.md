# 0054: How custom code fails

Tech spec for [proposal 0054](https://github.com/itinera-dev/spec/blob/main/proposals/0054-how-custom-code-fails.md), implemented in [#22](https://github.com/itinera-dev/itinera-rs/issues/22). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 1 adds the error type; the rest arrives with the executors, hooks and proofs.

## API

- **`itinera::error::Error`** is the error every piece of custom code returns: steps, hooks, role operations, policy factories, reporters, dispatchers and dispatcher factories.
  - It converts from any `std::error::Error + Send + Sync + 'static`, so `?` works on the errors of any library.
  - Because of that conversion it does not implement `std::error::Error`. It offers `Display`, `Debug`, `source()` and `Error::msg`, and converts into `Box<dyn std::error::Error + Send + Sync>`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| All custom code fails the same way, by returning an error | types: every trait for custom code returns `Result<_, Error>` |
| Unrecoverable failures are outside the model | panics are never caught |

## Tests

- Unit tests in `itinera-core/src/error.rs`: an error shows the message of the error it was made from, and converts back into a boxed standard error.

## Done when

Every scenario tagged `@proposal-0054` passes or is proven impossible, and 54 is listed in `conformance.json`. Planned for stage 7.
