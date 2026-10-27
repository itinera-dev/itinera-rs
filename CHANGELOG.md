# Changelog

Every release of the crates `itinera`, `itinera-core` and `itinera-macros`, which share one version. Each release says the version of the [Itinera specification](https://github.com/itinera-dev/spec) it implements, the tier, the capabilities claimed and the [conformance cases](https://github.com/itinera-dev/conformance) it passes; its conformance report is attached to its GitHub release. Before 1.0, a breaking change raises the minor version and a fix the patch version.

## [0.1.0-beta.4]

The fourth beta, which breaks code written for `0.1.0-beta.3`: specification 0.1.0, tier 1, capabilities `sync`, `async` and `testing`, cases `v0.1.0-rc.16`. It implements proposal 0116, on two input adapters with one name and on an input adapter or service provider attached to one step twice, and proposal 0119, on mistakes in choosing a scripted run's attempts. It does not claim the capability `value-conversions`, which holds every case of proposal 0113, since itinera keeps a value as it is and reads it back by its exact type, calling no conversion. The minimum supported Rust version is 1.85.

### Added

- `event::Supplier`, the input adapter, with the key it was resolving, or the service provider that failed while a step was built.
- `workflow::Violation::DuplicateAdapterName` and `workflow::ViolationKind::DuplicateAdapterName`: `workflow::WorkflowBuilder::build` reports two input adapters with the same name instead of `workflow::WorkflowBuilder::input_adapter` panicking.

### Changed

- `event::JourneyAbort::StepCouldNotBeBuilt` replaces its `provider` field with `supplier`, an `Option<event::Supplier>`, which holds `Supplier::Provider` where `provider` held a service provider's name.
- `workflow::InputAdapterDescriptor::step` and `service::ServiceProviderDescriptor::step` no longer ignore a step they are already attached to; `workflow::WorkflowBuilder::build` reports `workflow::Violation::StepAdaptedTwice` or `workflow::Violation::ServiceProvidedTwice`, naming the adapter or provider once per attachment.
- The compile errors of `#[workflow]` for two input adapters of one name, and for an adapter or provider listing a step twice, name the violations `duplicate adapter name`, `step adapted twice` and `service provided twice`.

### Fixed

- `journey_aborted` names the input adapter that failed and the key of the input it was resolving, in `event::JourneyAbort::StepCouldNotBeBuilt`.
- With the `testing` feature, `instance::StepScripting::on_attempt` names an attempt the step can never have, such as attempt 0 or one past what it can have, before an attempt scripted already or a gap, and every panic of `instance::StepScripting` in choosing attempts names the step and the attempt.

## [0.1.0-beta.3]

The third beta, which makes the API friendlier and breaks code written for `0.1.0-beta.2`: specification 0.1.0, tier 1, capabilities `sync`, `async` and `testing`, cases `v0.1.0-rc.15`. What a journey does is unchanged. The minimum supported Rust version is 1.85.

### Added

- `error::Result<T, E = Error>`, the result custom code returns, such as `Result<Outcome>` from a step's `run`. A service provider declared with `#[workflow]` may return `Result<Provided<T>>`.
- Readers on how a journey ended: `JourneyStatus::is_succeeded`, `is_failed`, `is_aborted`, `failure` and `abort`, and `Failure::reason` and `error`.
- `step::Attempt`, an attempt's number, counted from 1, with `Attempt::FIRST` and `is_first()`.
- `event::EventKind`, an event's kind, which displays as its snake_case name and parses from it.
- `workflow::Supplied`, what an input adapter returns, with `Supplied::value` and the reader `downcast_ref::<T>()`.
- `journey::Read::present`, the value read when it is present and of the type asked for.

### Changed

- Step hooks return `policy::OnSuccess` (`on step success`) or `policy::OnFailure` (`on step failure`, `on step retry` and `on step abnormal termination`) instead of an `Option` of a lifecycle: `KeepDefault`, `FinishWorkflow` for `OnSuccess` only, or `FailWorkflow` with a reason, made with `OnSuccess::fail_workflow` or `OnFailure::fail_workflow`, which take any text. They read themselves back with `keeps_default`, `fails_workflow` and, on `OnSuccess`, `finishes_workflow`. The shorthand `DescribedPolicy::carry_on()` is now `keep_default()`; the `ReturnedLifecycle` trait and the `FailWorkflow` struct are gone.
- Input adapters return `workflow::Supplied`, a `Value` or `FromDataBag`, instead of an `Option` of a value, and take the data bag as `&DataBag`, whose `read` is now public; `DataBagAccess` and `DataBag::access` are gone.
- Steps and hooks take the contributor as `&mut journey::Contributor`, with no lifetime.
- The macros recognise a handle by its type, so `#[contributor]`, `#[reporter]`, `#[journey_id]`, `#[step_name]`, `#[attempt]`, `#[reason]`, `#[error]`, `#[cause]` and `#[data_bag]` are gone. Marks stay where the type cannot say what is requested: `#[input]`, `#[service]`, `#[from_step]`, `#[from_workflow]`, `#[from_rescued_step]`, `#[key]` and `#[role]`.
- An attempt's number is a `step::Attempt` instead of a `NonZeroU32` wherever it is public; scripts still number attempts with plain integers.
- `Event::kind` and `EventBody::kind` return an `EventKind` instead of a `&'static str`.
- `Outcome::failure`, `Outcome::skipped_because` and `RescueOutcome::failure` take anything that converts into a `Reason`, so a text is the reason's code.
- In the builder form, each source of data has one request method, which takes an `Input` or an `OptionalInput`, through the sealed `step::InputToken`, and gives the value or an `Option` of it: `optional_input`, `optional_from_workflow`, `optional_from_step` and `optional_from_rescued_step` are gone.
- `Provides` and `InputAdapter` move from `policy` to `workflow`, and `Provider` to `service`.
- A release is published only once every CI check passes on its tagged commit, and docs.rs builds each crate with its features other than `unstable`.

## [0.1.0-beta.2]

The second beta, with the same implementation as `0.1.0-beta.1`: specification 0.1.0, tier 1, capabilities `sync`, `async` and `testing`, cases `v0.1.0-rc.15`. The minimum supported Rust version is 1.85.

### Changed

- On crates.io, the crates are listed under the categories `asynchronous` and `rust-patterns` and found by the keyword `agent` instead of `events`, and the pages of `itinera-core` and `itinera-macros` send readers to `itinera`.
- The crates are published by the release workflow, through crates.io's trusted publishing.

## [0.1.0-beta.1]

The first beta: specification 0.1.0, tier 1, capabilities `sync`, `async` and `testing`, cases `v0.1.0-rc.15`. The minimum supported Rust version is 1.85.

### Added

- Steps that declare the data they need and report an outcome. Only an attempt that succeeds commits its contributions to the data bag of values. A retry budget alone decides whether a failed or abnormally terminated attempt is tried again; a step that says `no_retry_on_abnormal_termination()` is not retried after an abnormal termination.
- Step and workflow policies, every hook kind with its requests and lifecycles, roles the workflow provides, and input adapters.
- Rescue steps: a step may have a rescue step, which runs when the step fails or terminates abnormally and, once it succeeds, lets the step run again. Input adapters and service providers are attached to it by the name `<step name>_rescue`.
- Services that steps request by key, made by service providers of the workflow for the journey or for each attempt, and attached to chosen steps, rescue steps included, or to no step, serving every step that requests their key and has no provider of its own for it. A provider returns its service made with `service::provide!`, which names the implementation it made, its concrete type, on the `service_provided` event, together with data the provider may add.
- Workflow descriptors declared with the builder and refused when they break a rule of the specification, and listed, with their steps, policies, input adapters, services and rescue steps, by `listing()`; workflow instances with their initial data, journey IDs and reporters, and the results and refusals of a journey.
- Events, reporters, dispatchers and their factories, with the default dispatcher, and two ready-made reporters: `DebugReporter`, which prints every event, and, with the `testing` feature, `RecordingReporter`, which keeps every event, and the latest value of each entry of global metadata, for a test to read.
- Global metadata: entries of a key and a value, apart from the data bag, that every reporter receives as soon as each one is set, starting with the journey ID under the key `journey_id`. A workflow instance may start with more entries, and steps, hooks and service providers may set entries while the journey runs, each recorded by a `global_metadata_set` event that carries its key but not its value.
- Test support, with the `testing` feature: `RecordingReporter::path()`, which lists the flow a recorded journey took, one plain line per attempt's outcome, decision and end; `fake_provider`, on the instance builder and on the builder `#[workflow]` generates, which puts a fake in place of a service provider for one instance; scripted runs, `instance.scripted()`, which play what named steps' attempts do, without their business rules, and record what each attempt was built with; and recorders that let a test call a step's `run`, a hook or an input adapter directly, outside any journey: `journey::Contributions` lends a contributor, `step::StepEventReporter` and `policy::HookEventReporter` lend the reporters and keep what is emitted and the global metadata that was set, and `journey::DataBag::default().with(key, value)` builds a data bag whose `access()` an input adapter reads.
- Step and rescue step outcomes can be read back with `is_success`, `is_failure` and `reason`, and a step's outcome also with `is_skipped`; what a hook returned, with `keeps_default`, `fails_workflow` and `finishes_workflow` from `policy::ReturnedLifecycle`.
- The local executor and, with the `async` feature, the asynchronous local executor, and `executor::local_executor_for!`, which makes the local executor of a workflow's mode.
- With the `async` feature, `mode::async_trait`, the attribute for services and roles whose methods are asynchronous.
- With the `macros` feature, on by default, attributes and derives that declare steps, rescue steps, policies, workflows with their service providers, roles and journey data as syntax over the builder. A workflow declared with `#[workflow]` takes its mode from its steps and refuses steps of both modes, and `#[service]` takes a service only as a trait object, such as `&dyn PaymentGateway`, refusing a concrete type.
- Compile errors, instead of checks while a journey runs, for the rules the specification lets an implementation make impossible to express: a lifecycle a hook may not return, a role the workflow does not provide, an asynchronous part on the synchronous executor or an asynchronous service provider in a synchronous workflow, a rescue step that skips, a service provider of journey scope requesting the step being built or the attempt, a service provider returning nothing, a non-value entering the data bag, event data, a reason's details or global metadata, handles used after their attempt or call, and an input adapter writing to the data bag.

[0.1.0-beta.4]: https://github.com/itinera-dev/itinera-rs/releases/tag/v0.1.0-beta.4
[0.1.0-beta.3]: https://github.com/itinera-dev/itinera-rs/releases/tag/v0.1.0-beta.3
[0.1.0-beta.2]: https://github.com/itinera-dev/itinera-rs/releases/tag/v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/itinera-dev/itinera-rs/releases/tag/v0.1.0-beta.1
