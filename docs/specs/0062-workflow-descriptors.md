# 0062: Workflow descriptors, and executors independent of how workflows are written

Tech spec for [proposal 0062](https://github.com/itinera-dev/spec/blob/main/proposals/0062-workflow-descriptors.md), implemented in [#29](https://github.com/itinera-dev/itinera-rs/issues/29). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 4. Stage 3 added the workflow descriptor with its name, its reporters and its journey ID generator, and the instance trait executors rely on. Stage 4 added step and policy descriptors, input adapters, admission and the listing. Some of what the proposal asks is met only by the stand-ins described below, until stages 5 and 7 replace them; the table of rules says which.

## API

- **`itinera::workflow::WorkflowDescriptor<W, M>`** is the workflow's declaration. `W` is the workflow's own type and `M` its execution mode, `itinera::mode::Synchronous` unless an asynchronous part makes it `itinera::mode::Asynchronous`. It is cheap to clone, since clones share one declaration behind an `Arc`.
- **`WorkflowDescriptor::builder(name)`** returns a `WorkflowBuilder`. Its `step`, `policy` and `input_adapter` add the workflow's step descriptors in order, its workflow policy descriptors in order, and its input adapters. Only its `build()` makes a descriptor, and it returns `Result<WorkflowDescriptor<W, M>, Violations>`.
- **`itinera::step::StepDescriptor`** holds a step's name, how to run it, and the step policies attached to it in order, with `StepDescriptor::new(name, run)` and `.policy(descriptor)`.
- **`itinera::policy::StepPolicyDescriptor`** and **`itinera::policy::WorkflowPolicyDescriptor`** describe one policy each: its name and the hooks it defines, of its own kind only, at least one. They are cloned to be attached to several steps or workflows.
- **`itinera::workflow::InputAdapter`** declares an input adapter: its name and the steps it is attached to, at least one.
- **Names** are `itinera::workflow::WorkflowName`, `itinera::step::StepName`, `itinera::policy::PolicyName` and `itinera::workflow::AdapterName`, each over a `&'static str`, so they are `Copy`. `StepName::new` panics on an empty name, and `itinera::step::step_name!` calls it in a `const` block, so an empty literal does not compile.
- **`WorkflowDescriptor::listing()`** returns a `ListedStep` per step, in order: its name, its position from 1 as a `NonZeroUsize`, its policies' names in order, and its input adapter's name, if any.
- **`itinera::workflow::Violations`** holds every `Violation` found by `build()`, at least one, and implements `std::error::Error`. `Violation::kind()` gives the plain `ViolationKind`, which displays as the specification names it.
- **`itinera::instance::WorkflowInstance`** is the only thing an executor relies on, so an executor runs an `Instance<W, M>` and a hand-written instance alike.
- **`itinera::mode::Mode`** is sealed: `Synchronous` and `Asynchronous` are the only modes.

### Stand-ins until stages 5 and 7

- A step is a closure that can only succeed. Stage 5 replaces it with a `StepFactory`.
- A policy descriptor names the hooks it defines, with `StepHook` or `WorkflowHook` values, and they keep their default behaviour. Stage 7 replaces the names with the policies' hooks and a factory.
- An input adapter supplies no value. Stage 7 makes it a hook of the workflow.
- A step's retry budget and `abnormal termination retriable` arrive with stage 6, which uses them.

While these stand-ins remain, and while the API names reporters, instances and executors of proposals not yet listed, it stays behind the `unstable` feature, though 62 is listed in `conformance.json`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A workflow descriptor does not change once built | types: it has no method that changes it, and its declaration is shared behind an `Arc` |
| It is the same for every instance | types: every instance holds a clone of the one descriptor |
| A descriptor holds descriptions and a way to build each step and policy, never instances | types: it holds step and policy descriptors only. Until stage 5, a step descriptor holds the step's closure rather than a way to build the step; until stage 7, a policy descriptor holds no way to build the policy, nor what its hooks request or their execution mode |
| The executor builds steps and policies; an instance never builds, holds or orders them | types: an instance holds only a clone of the descriptor, and the engine runs the steps in order. Building per attempt arrives with stage 5 for steps and stage 7 for policies |
| Admission applies to every descriptor before a journey starts | types: only `build()` makes a descriptor, and it checks the declaration first |
| An executor does not depend on how a workflow, its descriptor or its instance was produced | types: executors take any `WorkflowInstance` |
| The listing reads the descriptor without running anything | `listing()` reads only the declaration |
| A step's name is non-empty | `StepName::new` panics; `step_name!` makes it a compile error |
| An input adapter's name is unique among the workflow's adapters | `WorkflowBuilder::input_adapter` panics; the macros declare adapters as methods, whose names are unique |
| `duplicate step name`, `hook defined twice`, `step adapted twice` and `input adapter for unknown step`, all reported together | `build()` |
| A policy defines hooks of one kind only, and at least one; an input adapter is attached to at least one step | types: each descriptor's constructor takes its first hook or step |

## Tests

- The unit tests of `itinera-core/src/workflow/violation.rs` refuse each violation, report every violation together, build a well-formed workflow, and refuse a second adapter with a used name.
- The unit tests of `itinera-core/src/workflow.rs` list a workflow without running anything, and those of `itinera-core/src/step.rs` refuse an empty step name.
- The unit tests of `itinera-core/src/engine.rs` run steps in the order they were added.
- The conformance runner defines "When the workflow is admitted", "When the workflow is listed", "Then the listing is", "Then admission is refused with the violations" and "Then no step ran". Its fixture `tests/cases/admission/` checks them, and its unit tests refuse an empty step name and a second input adapter with a used name as errors in the case.

## Done when

Every scenario tagged `@proposal-0062` passes, and 62 is listed in `conformance.json`. The cases tag no scenario with `@proposal-0062`, since the proposal is not observable on its own, so listing it runs nothing new.
