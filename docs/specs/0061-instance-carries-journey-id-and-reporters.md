# 0061: The workflow instance carries its journey ID and its reporters

Tech spec for [proposal 0061](https://github.com/itinera-dev/spec/blob/main/proposals/0061-instance-carries-journey-id-and-reporters.md), implemented in [#28](https://github.com/itinera-dev/itinera-rs/issues/28). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: in progress. Stage 3 adds the instance, its journey ID and its reporters, and the executors that use them.

## API

- **`itinera::instance::WorkflowInstance`** is what executors run. It gives the descriptor, the workflow's own value, the `journey_id`, the reporters through `take_reporters`, and the data bag.
- **`itinera::instance::Instance<W, M>`** is the instance itinera makes, with `WorkflowDescriptor::instance(workflow)`, then `.data(key, value)` for the initial data, then `.create()`.
- **The journey ID generator** is given with `WorkflowBuilder::id_generator`, a closure from the workflow's own value and the initial data to `Result<String, Error>`. Without one, the ID is a UUID v4, built by the `uuid` crate from random bytes that `getrandom` reads; a random source that fails is an `InstanceError`, never a panic.
- **Reporters** are listed by type with `WorkflowBuilder::reporter::<R>()`, where `R: WorkflowReporter<W>`, or with `async_reporter::<R>()`, where `R: AsyncWorkflowReporter<W>`. Each has `init(&W, &JourneyId, &DataBag) -> Result<Self, Error>`.
- **`itinera::instance::InstanceError`** is what `create()` returns when the generator or a reporter's `init` fails: `JourneyId(Error)` or `Reporter(Error)`.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| The instance is created with its initial data, then its journey ID, then its reporters | `InstanceBuilder::create` |
| Without a generator, the journey ID is a UUID v4 | the descriptor's default |
| The generator MAY derive the ID from the instance's data | types: it receives the initial data as `&DataBag` |
| Reporters are made after the journey ID, and MAY receive it | types: `init` receives the `JourneyId` |
| A failure while the instance is created happens before the executor is involved | types: `create()` returns `InstanceError`, and no instance exists to run |

## Tests

- Unit tests in `itinera-core/src/instance.rs`: without a generator the journey ID is a UUID v4; the generator produces the journey ID from the initial data; a generator that fails, or a reporter that cannot be made, makes creating the instance fail; reporters are made after the journey ID with the initial data; the instance holds the workflow's value and its initial data; an asynchronous workflow makes reporters of both kinds in the order they are listed.

## Done when

Every scenario tagged `@proposal-0061` passes, and 61 is listed in `conformance.json`. Planned for stage 6.
