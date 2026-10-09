# 0057: Contributors and step reporters are valid only during their attempt

Tech spec for [proposal 0057](https://github.com/itinera-dev/spec/blob/main/proposals/0057-handles-valid-during-their-attempt.md), implemented in [#25](https://github.com/itinera-dev/itinera-rs/issues/25). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 5 for steps. Stage 7 gives hooks their own contributor and reporter, which follow the same rule.

## API

- **`itinera::journey::Contributor<'a>`** and **`itinera::step::StepReporter<'a>`**, with **`itinera::step::AsyncStepReporter<'a>`** behind the `async` feature, are the handles of one attempt. `'a` is the attempt's lifetime, chosen by the engine.
- A step declares them with `StepNeeds::contributor()` and `StepNeeds::reporter()`, and its factory takes them from `Resolved` when it builds the step. The step keeps them as fields, and `run(self)` consumes the step and its handles with it.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| The engine makes the handles for one attempt | types: only the engine can make a handle, and it makes new ones for every attempt |
| A handle cannot be used once its attempt has ended | types: a handle borrows its attempt, so the borrow checker refuses any code that keeps it beyond `'a`, in the factory, a `static`, or a thread that may outlive the attempt |
| During its attempt, a step contributes and emits freely, in any order | the handles take `&mut self`, and the step owns them |

## Tests

- Unit tests in `itinera-core/src/engine.rs`: what a step contributes and emits through its handles during its attempt reaches the data bag and the events.

## Excluded scenarios

The scenario of this proposal, "A step's contributor and reporter used after its attempt change nothing", is tagged `@late-handle`, and proven impossible to express by `a_steps_handles_cannot_outlive_its_attempt` in `crates/itinera/tests/proofs.rs`. It shows a factory that keeps both handles beyond the attempt and a step that moves them into a thread that may outlive it, neither of which compiles, and a step that uses them from a scoped thread, which ends within the attempt and compiles.

## Done when

Every scenario tagged `@proposal-0057` is proven impossible to express, and 57 is listed in `conformance.json`. Done in stage 5.
