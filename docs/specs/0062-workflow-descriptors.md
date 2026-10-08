# 0062: Workflow descriptors, and executors independent of how workflows are written

Tech spec for [proposal 0062](https://github.com/itinera-dev/spec/blob/main/proposals/0062-workflow-descriptors.md), implemented in [#29](https://github.com/itinera-dev/itinera-rs/issues/29). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 3 adds the workflow descriptor with its name, its reporters and its journey ID generator, and the instance trait executors rely on; stage 4 adds step and policy descriptors, input adapters and the listing.

## API

- **`itinera::workflow::WorkflowDescriptor<W, M>`** is the workflow's declaration. `W` is the workflow's own type and `M` its execution mode, `itinera::mode::Synchronous` unless an asynchronous part makes it `itinera::mode::Asynchronous`. It is cheap to clone, since clones share one declaration behind an `Arc`.
- **`WorkflowDescriptor::builder(name)`** returns a `WorkflowBuilder`, and only its `build()` makes a descriptor.
- **`itinera::instance::WorkflowInstance`** is the only thing an executor relies on, so an executor runs an `Instance<W, M>` and a hand-written instance alike.
- **`itinera::mode::Mode`** is sealed: `Synchronous` and `Asynchronous` are the only modes.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A workflow descriptor does not change once built | types: it has no method that changes it, and its declaration is shared behind an `Arc` |
| It is the same for every instance | types: every instance holds a clone of the one descriptor |
| An executor does not depend on how a workflow, its descriptor or its instance was produced | types: executors take any `WorkflowInstance` |

## Tests

- The unit tests of `itinera-core/src/instance.rs` create instances from descriptors built with the builder.

## Done when

Every scenario tagged `@proposal-0062` passes, and 62 is listed in `conformance.json`. Planned for stage 4.
