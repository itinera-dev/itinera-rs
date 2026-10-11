# 0095: Services provided to steps

Tech spec for [proposal 0095](https://github.com/itinera-dev/spec/issues/95), implemented in [#103](https://github.com/itinera-dev/itinera-rs/issues/103) as stage 10 of the [tier 1 plan](../tier-1-plan.md), which holds what crosses proposals. It amends [0002](0002-foundations.md), [0003](0003-rescue-steps.md), [0011](0011-events.md) and [0042](0042-configuration-errors-before-the-journey.md).

Status: in progress. Its API is behind the `unstable` feature until 0095 is accepted and listed in `conformance.json`.

## API

Everything below is in `itinera::service`, unless another module is named.

- **`Service<T: ?Sized>`**, a typed token holding a key, declared as a constant like `Input<T>`: `const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");`. `T` is any `Send + Sync + 'static` type, a trait object or a concrete type.
- **The step's side.** `StepNeeds::service(&PAYMENTS)` declares a service, and `Resolved::service(&PAYMENTS)` takes it, as `Result<&'a T, Error>`, where `'a` is the attempt. A step may take a service more than once. Taking one it did not declare, or as another type, fails. A rescue step does the same with `RescueNeeds::service(&PAYMENTS)` and `RescueResolved::service(&PAYMENTS)`.
- **`ProviderName`**, a `&'static str` newtype like `AdapterName`.
- **The scopes** `PerJourney` and `PerAttempt` are types, and implement the sealed trait `ProviderScope`, whose sealing trait tells the executor which scope it is, so a descriptor does not store it.
- **`ServiceProviderDescriptor<W, S: ProviderScope, M = Synchronous>`** describes a provider:
  - `::new(name, &TOKEN, provide)` makes one of scope `PerJourney`, and `::per_attempt(name, &TOKEN, provide)` one of scope `PerAttempt`. With the `async` feature, an asynchronous workflow's providers are made with `::new_async` and `::per_attempt_async`, which take an `async fn` as well as a function returning a future (`AsyncProvide`);
  - `provide` is a function of the workflow's own value and a `Requested<'_, W, Provider<S>, M>`, returning `Result<Box<T>, Error>`, usually a method of the workflow;
  - `.needing(HookNeeds<Provider<S>>)` declares its requests, and `.step(name)` attaches it to a step. `.name()` gives its name.
- **`itinera::policy::Provider<S>`** is the hook kind of a provider. `Requested` gives it the journey ID, data from the workflow and a reporter. One of scope `PerAttempt` also has `step_name()` and `attempt()`.
- **`itinera::policy::ReportingHookKind`**, a trait every policy hook kind and `Provider<S>` implement, carries `HookNeeds::reporter()` and `Requested::reporter()`. Two inherent `reporter` methods on overlapping impls would break type inference in existing code. It is public only with `unstable`; without it, `ReportingHookKind` is a crate-private name for `PolicyHookKind`, so the reporter methods stay where they were.
- **`WorkflowBuilder::provider(descriptor)`** declares a provider on the workflow. The descriptor's mode is the workflow's, so a synchronous workflow cannot be given an asynchronous provider.
- **The listing.** `ListedStep::services` lists the services each step requests, as `ListedService { key, provider }`, in the order the step declares them.
- **Events.** `EventBody::ServiceProvided` and `EventBody::ServiceProviderFailed` carry the step's attempt, the key and the provider. `HookSource::Provider` names the provider and the step's attempt, for its `journey_*` events, and, within `RequestSource::Hook`, for its requests. `event::Requester::Provider` and `journey::Requester::Provider` name it in aborts. `JourneyAbort::StepCouldNotBeBuilt` names the provider when a provider failed.
- **Violations.** `ServiceNotProvided`, `ServiceRequestedTwice`, `ServiceProvidedTwice`, `DuplicateProviderName`, `ServiceProviderForUnknownStep` and `WrongServiceType`, with a `ViolationKind` of each.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Only steps request services | types: only `StepNeeds` and `RescueNeeds` have `service`; hooks and adapters have no way to request one |
| A service is requested by key and type, and is always required | types: `Service<T>` carries both; `Resolved::service` returns `Result<&T, Error>`, failing only for a service the step did not declare |
| Which provider serves a step: the one of that key attached to it, else the one attached to no step | `serving` in `service/descriptor/entry.rs`, used by admission, the listing and the engine alike |
| `service not provided`, `service requested twice`, `service provided twice`, `duplicate provider name`, `service provider for unknown step`, `wrong service type` | `build()`, in `workflow/violation/admission/services.rs`, and, for what each step and rescue step requests, in `workflow/violation/admission/services/requests.rs`; the type is compared by the `TypeId` of `Box<T>` |
| `attempt request in journey scope` | types: `step_name()` and `attempt()` exist only on `Requested<'_, W, Provider<PerAttempt>, M>`; see the excluded scenarios |
| A provider may request the journey ID, data from the workflow and a reporter, and nothing else | types: `HookNeeds<Provider<S>>` and `Requested<'_, W, Provider<S>, M>` offer only those, since `Provider<S>` is neither a step hook kind nor a policy hook kind |
| Services are resolved after the step's inputs, in the order the step declares them, before it is built | the engine, in `engine/attempt.rs` and `engine/services.rs` |
| Journey scope: the provider is called the first time a step it serves needs its key, and that instance is lent to every later attempt of the steps it serves | the engine keeps one `OnceLock` slot per journey-scoped provider, in `engine/services/kept.rs`, in an `Arc` the journey owns, dropped when the run ends |
| Attempt scope: the provider is called for every attempt, and its instance serves that attempt only | the engine owns the instance while the step is built and runs, and drops it when the attempt ends, in `engine/attempt.rs` |
| A provider's requests, events and failure: `optional_input_absent`, `required data missing` and `wrong type` naming it, `service_provided`, `service_provider_failed`, `step could not be built` naming it | the engine, in `engine/services.rs`, through the same request resolution and reporting as a hook's |
| A service is lent for its attempt only | types: `Resolved::service` returns a reference bound to the attempt; no conformance case checks a kept service |
| A provider's reporter is valid only during its call | types: its lifetime is the call's, as a hook's reporter is |
| A provider returns an instance or an error, never nothing | types: its function returns `Result<Box<T>, Error>`; see the excluded scenarios |
| A provider follows the executor's execution modes | types: its mode is the workflow's; see the excluded scenarios |
| Event data a provider emits is a value | types: the reporter takes `Value`s only, as a hook's does |
| A rescue step is served only by providers attached to no step | `serving` in `service/descriptor/entry.rs`, given `Served::RescueStep`, looks for no provider attached to the step. Admission checks each rescue step's services as a step's, with `Served::RescueStep`, in `services/requests.rs`, so a service that only a provider attached to a step supplies is `service not provided`; the engine and the listing ask the workflow, whose `Declaration::served` takes a name it declares no step under for a rescue step's |
| A provider attached to a rescue step's name is `service provider for unknown step` | admission looks for a provider's steps among the declared steps only, in `workflow/violation/admission/services.rs` |

## Tests

- Unit tests in `itinera-core/src/workflow/violation/admission/services.rs`: `a_workflow_whose_providers_are_declared_wrongly_is_refused_with_its_violation`, with a case per violation of the providers, among them `a_provider_attached_to_a_rescue_steps_name`, `a_provider_attached_to_a_step_that_does_not_request_its_service_is_allowed` and `a_provider_attached_to_a_rescue_steps_name_never_serves_it`.
- Unit tests in `itinera-core/src/workflow/violation/admission/services/requests.rs`: `a_workflow_serving_requested_services_wrongly_is_refused_with_its_violation`, with a case per violation of what steps and rescue steps request, `a_provider_attached_to_a_step_replaces_the_one_attached_to_no_step_for_it` and `a_rescue_step_is_served_by_the_provider_attached_to_no_step_not_by_the_rescued_steps`. Their shared fixtures are in `services/fixtures.rs`.
- Unit tests in `itinera-core/src/service/descriptor/entry.rs`, on `serving`: `a_provider_attached_to_a_step_serves_it_and_the_one_attached_to_no_step_serves_the_rest` and `a_rescue_step_is_served_only_by_the_provider_attached_to_no_step`.
- Unit tests in `itinera-core/src/engine/services/tests.rs`, one per rule of scope, attachment, requests, events, failures and the order of a step's services, and one with an asynchronous provider; and, in its module `rescue_steps`, `a_rescue_step_is_served_by_the_provider_attached_to_no_step`, `an_attempt_scoped_provider_is_told_the_rescue_steps_name_and_its_attempt_1` and `a_journey_scoped_provider_lends_a_rescue_step_the_journeys_instance`.
- Unit tests in `itinera-core/src/engine/attempt/tests.rs`: `services_are_supplied_after_the_steps_inputs_are_resolved` and `an_attempt_scoped_service_is_dropped_when_its_attempt_ends`; and in `itinera-core/src/engine/tests.rs`: `a_journey_scoped_service_is_dropped_when_the_journey_ends`.
- Unit tests in `itinera-core/src/step/resolved.rs`, on taking a service, in `itinera-core/src/workflow/listing.rs`, on the listing, and in `itinera-core/src/workflow/violation/kind.rs` and `itinera-core/src/workflow/violation.rs`, on the violations' names.
- The shared fixtures, a service that tells its label, one that counts its instances dropped, their providers, and steps and rescue steps that request them, are in `itinera-core/src/service/fixtures.rs`.

## Excluded scenarios

| Tag | Proof in `crates/itinera/tests/proofs.rs` |
|---|---|
| `@journey-scope-request` | `a_journey_scoped_provider_cannot_request_the_attempt`: a journey-scoped provider that calls `attempt()` does not compile; the twin, an attempt-scoped one, does |
| `@no-instance` | `a_provider_cannot_return_nothing`: a provider returning `Result<(), Error>` does not compile; the twin returns a `Box` |
| `@mode-not-accepted`, for a provider | `a_synchronous_workflow_cannot_have_an_asynchronous_provider`: an asynchronous provider given to a synchronous workflow does not compile; the twin gives it to an asynchronous workflow |
| `@non-value`, for a provider's event | `a_providers_event_cannot_carry_a_non_value`: a provider emitting a closure as event data does not compile; the twin emits a number |

The scenarios are listed under `impossible` in `conformance.json` when the cases holding them are pinned.

## Done when

Every scenario tagged `@proposal-0095` passes or is proven impossible to express, 95 is listed in `conformance.json`, and its API is out of `unstable`.
