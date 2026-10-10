# 0062: Workflow descriptors, and executors independent of how workflows are written

Tech spec for [proposal 0062](https://github.com/itinera-dev/spec/blob/main/proposals/0062-workflow-descriptors.md), implemented in [#29](https://github.com/itinera-dev/itinera-rs/issues/29). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done in stage 4. Stage 3 added the workflow descriptor with its name, its reporters and its journey ID generator, and the instance trait executors rely on. Stage 4 added step and policy descriptors, input adapters, admission and the listing. Stage 5 replaced the step's closure with a factory that builds the step for each attempt, and made input adapters supply values. Stage 6 added the step's retry budget and `abnormal termination retriable`. Stage 7 gave policy descriptors their factories and their hooks, with what each hook requests. Some of what the proposal asks is met only by the stand-in described below, until stage 7 replaces it.

## API

- **`itinera::workflow::WorkflowDescriptor<W, M>`** is the workflow's declaration. `W` is the workflow's own type and `M` its execution mode, `itinera::mode::Synchronous` when it is declared with `WorkflowDescriptor::builder`, and `itinera::mode::Asynchronous` with `WorkflowDescriptor::async_builder`. It is cheap to clone, since clones share one declaration behind an `Arc`.
- **`WorkflowDescriptor::builder(name)`** returns a `WorkflowBuilder`. Its `step`, `policy` and `input_adapter` add the workflow's step descriptors in order, its workflow policy descriptors in order, and its input adapters. Only its `build()` makes a descriptor, and it returns `Result<WorkflowDescriptor<W, M>, Violations>`.
- **`itinera::step::StepDescriptor<W, M>`** holds a step's name, what it needs, its factory, its retry budget, whether an abnormal termination may be retried, and the step policies attached to it in order, with `StepDescriptor::new(name, factory)` for a `StepFactory`, `StepDescriptor::new_async(name, factory)` for an `AsyncStepFactory`, `.retry_budget(n)` with a `u16`, `.abnormal_termination_retriable()` and `.policy(descriptor)`. The budget is 0, and an abnormal termination is not retried, unless set.
- **`itinera::policy::StepPolicyDescriptor`** and **`itinera::policy::WorkflowPolicyDescriptor`** describe one policy each: its name, the factory the executor builds it with, and the hooks it defines, of its own kind only, each with what it requests. Their constructors return a `Hookless` descriptor, and naming a hook, with `.on_step_success()` or `.on_step_success_needing(needs)` and the like, makes it `Hooked`; only a `Hooked` descriptor can be attached, so every attached policy defines at least one hook.
- **`itinera::workflow::InputAdapter<W>`** declares an input adapter: its name, the steps it is attached to, at least one, and its function of the workflow's own value, the step's name and the key.
- **Names** are `itinera::workflow::WorkflowName`, `itinera::step::StepName`, `itinera::policy::PolicyName` and `itinera::workflow::AdapterName`, each over a `&'static str`, so they are `Copy`. `StepName::new` panics on an empty name, and `itinera::step::step_name!` calls it in a `const` block, so an empty literal does not compile.
- **`WorkflowDescriptor::listing()`** returns a `ListedStep` per step, in order: its name, its position from 1 as a `NonZeroUsize`, its policies' names in order, and its input adapter's name, if any.
- **`itinera::workflow::Violations`** holds every `Violation` found by `build()`, at least one, and implements `std::error::Error`. `Violation::kind()` gives the plain `ViolationKind`, which displays as the specification names it.
- **`itinera::instance::WorkflowInstance`** is the only thing an executor relies on, so an executor runs an `Instance<W, M>` and a hand-written instance alike. Its data operation `data_for_step(step, key)` answers with a `Resolution`: the value the step's input adapter supplied, the adapter's failure, the value in the data bag, or nothing. Its default is itinera's own, which asks the adapter first and falls back to the data bag. Its `commit(key, value)` writes a successful attempt's contribution to the data bag and answers `Committed::Added` or `Committed::Overwritten`, from which the engine emits `data_overwritten`.
- **`itinera::mode::Mode`** is sealed: `Synchronous` and `Asynchronous` are the only modes.

### Stand-in until stage 7

- An input adapter is a function of the workflow's own value, the step's name and the key, and requests nothing. Stage 7 makes it a hook of the workflow, with its requests.

While this stand-in remains, or while the API names that of proposals not yet listed, such as the events of 0011 and the policies of 0010, it stays behind the `unstable` feature, though 62 is listed in `conformance.json`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A workflow descriptor does not change once built | types: it has no method that changes it, and its declaration is shared behind an `Arc` |
| It is the same for every instance | types: every instance holds a clone of the one descriptor |
| A descriptor holds descriptions and a way to build each step and policy, never instances | types: it holds step and policy descriptors only, and a step descriptor holds its factory. A policy descriptor holds the policy's factory and what its hooks request; its mode is the workflow's |
| The executor builds steps and policies; an instance never builds, holds or orders them | types: an instance holds only a clone of the descriptor, and the engine builds each step for each attempt, from its factory, and runs the steps in order. It builds the workflow's policies once per journey and a step's policies for each attempt |
| Admission applies to every descriptor before a journey starts | types: only `build()` makes a descriptor, and it checks the declaration first |
| An executor does not depend on how a workflow, its descriptor or its instance was produced | types: executors take any `WorkflowInstance` |
| The listing reads the descriptor without running anything | `listing()` reads only the declaration |
| A step's name is non-empty | `StepName::new` panics; `step_name!` makes it a compile error |
| An input adapter's name is unique among the workflow's adapters | `WorkflowBuilder::input_adapter` panics; the macros declare adapters as methods, whose names are unique |
| `duplicate step name`, `hook defined twice`, `step adapted twice` and `input adapter for unknown step`, all reported together | `build()` |
| A policy defines hooks of one kind only, and at least one; an input adapter is attached to at least one step | types: a policy descriptor offers only the hooks of its kind, and only a `Hooked` one, which has named a hook, can be attached; an input adapter's constructor takes its first step |

## Tests

- The unit tests of `itinera-core/src/workflow/violation.rs` refuse each violation, report every violation together, build a well-formed workflow, and refuse a second adapter with a used name.
- The unit tests of `itinera-core/src/workflow.rs` list a workflow without running anything, and those of `itinera-core/src/step.rs` refuse an empty step name.
- The unit tests of `itinera-core/src/engine.rs` run steps in the order they were added.
- The conformance runner defines "When the workflow is admitted", "When the workflow is listed", "Then the listing is", "Then admission is refused with the violations" and "Then no step ran". Its fixture `tests/cases/admission/` checks them, and its unit tests refuse an empty step name and a second input adapter with a used name as errors in the case.

## Done when

Every scenario tagged `@proposal-0062` passes, and 62 is listed in `conformance.json`. The cases tag no scenario with `@proposal-0062`, since the proposal is not observable on its own, so listing it runs nothing new.
