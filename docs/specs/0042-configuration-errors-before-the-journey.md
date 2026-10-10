# 0042: Configuration errors are caught before a journey starts

Tech spec for [proposal 0042](https://github.com/itinera-dev/spec/blob/main/proposals/0042-configuration-errors-before-the-journey.md), implemented in [#20](https://github.com/itinera-dev/itinera-rs/issues/20). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 6. Stage 4 added admission, which refuses a workflow put together wrongly when it is built. The execution mode was checked by types from stage 3; stage 6 proves it.

## API

- **`itinera::workflow::WorkflowBuilder::build`** returns `Result<WorkflowDescriptor<W, M>, Violations>`. `Violations` holds every violation found, each with its `ViolationKind`; the tech spec of 0062 says which.
- **The execution mode** is part of the workflow's type: `WorkflowDescriptor<W, M>`, with `M` either `Synchronous` or `Asynchronous`. `LocalExecutor::run` accepts only instances of `Synchronous` workflows, and `AsyncLocalExecutor::run` only instances of `Asynchronous` ones.

Its API stays behind the `unstable` feature, since it names API of proposals not yet listed, such as the events of 0011 and the policies of 0010, until stage 7 lists them.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Declaration violations are reported when the workflow is built, every one of them | `WorkflowBuilder::build` |
| No journey starts, the ID generator is not called and no event is emitted | types: a refused workflow has no descriptor, so no instance can be created and nothing can run |
| `mode not accepted` is caught before the journey starts | types: an instance of a workflow whose mode the executor does not accept does not compile as an argument to its `run` |

The violation `mode not accepted` can never occur at run time, so `ViolationKind` has no variant for it. Nor has `role not provided`: roles arrive in stage 7, where types make that violation impossible to express, and its proof belongs to the tech spec of 0010.

## Tests

- Unit tests in `itinera-core/src/workflow/violation.rs`: each violation of the declaration is refused, and every violation is reported together.
- A proof in `itinera/tests/proofs.rs`, `a_synchronous_executor_cannot_run_an_asynchronous_step`: handing `LocalExecutor` an instance of an asynchronous workflow fails to compile, and its twin, which hands it to `AsyncLocalExecutor`, compiles and runs.
- The conformance scenario of 0032 tagged `@proposal-0042`: a workflow with two steps of the same name is refused, with no event and no journey ID.

## Excluded scenarios

The scenario "A synchronous-only executor refuses an asynchronous step before the journey starts", of `cases/tier-1/0012-local-executor/`, is tagged `@mode-not-accepted`, and proven impossible to express by `a_synchronous_executor_cannot_run_an_asynchronous_step` in `crates/itinera/tests/proofs.rs`.

## Done when

Every scenario tagged `@proposal-0042` passes or is proven impossible to express, and 42 is listed in `conformance.json`. Done in stage 6, with the entry for `mode-not-accepted` under `impossible`.
