# 0104: The implementation a service provider made, and its data

Tech spec for [proposal 0104](https://github.com/itinera-dev/spec/blob/main/proposals/0104-service-implementation-on-events.md), implemented in [#134](https://github.com/itinera-dev/itinera-rs/issues/134). It amends [0011](0011-events.md) and [0095](0095-services-provided-to-steps.md). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done.

## API

- **`itinera::service::Provided<T: ?Sized>`** is what a service provider returns for the service `T`, in place of `Box<T>`: the instance, boxed as `T`, its `Implementation`, and the data the provider supplied, if any. `service()` lends the instance as `&T`, `implementation()` gives its implementation and `data()` its data, as an `Option<&AnyValue>`. A provider's function returns `Result<Provided<T>, Error>`, and so does a method marked `#[provider]`.
- **`provide!(instance)`** and **`provide!(instance, data)`**, exported as `itinera::service::provide`, make a `Provided`. The macro reads the name of the instance's concrete type with the hidden `Implementation::__of`, which calls `std::any::type_name_of_val`, while the value still has that type, then boxes it and passes both to the hidden `Provided::__new`. Rust turns that box into a box of a trait object, such as `Box<dyn PaymentGateway>`, only where the expected type is known: in a provider's return position, as in `Ok(provide!(Sandbox))`, or in a `let` that names its type, as in `let made: Provided<dyn PaymentGateway> = provide!(Sandbox);`. A function cannot do the same on stable Rust, since it cannot unsize a generic `Box<V>` into a `Box<T>`, so this is a macro. The data must be a `Value`; the macro passes it to `AnyValue::new`.
- **`Provided::from_box(boxed)`** makes a `Provided` of an instance that is already a `Box<T>`, such as one a library made, and **`Provided::with_data(data)`** supplies data with any `Provided`. The box hides the instance's concrete type, so `from_box` names `T`, the declared type, as the specification allows a language that cannot tell; its documentation asks the provider to say which implementation it made in its data.
- **`itinera::service::Implementation`** is the implementation's name, a `&'static str` newtype with `Display`, `Into<&'static str>` and `AsRef<str>`, and no `From`, so that no text of the application's becomes one. Its format is the compiler's, such as `orders::Sandbox`, and is for reading: reporters should not depend on it. It lives in `service/implementation.rs`, and `provide!` in `service/provided/provide.rs`.
- **`Provided::data()`** is public so that an application can test its own providers: it reads back what the provider returned, never data of the journey, and no step, hook or provider is ever given another provider's `Provided`.
- **`EventBody::ServiceProvided`** gains `implementation: Implementation` and `data: Option<AnyValue>`.

## The implementation's name in Rust

With `provide!`, Rust names the instance's concrete type: the macro takes the value itself, which is sized, so `type_name_of_val` sees its type before the box hides it. With `Provided::from_box`, the instance is already behind the box, so Rust cannot tell its concrete type and names the type the provider declares, `T`, such as `dyn PaymentGateway`, as the specification allows; that is the only case. A value given to `provide!` that is itself a pointer to a trait object, such as an `Arc<dyn PaymentGateway>` given where the `Arc` implements the trait, is named as that pointer type, which is the concrete type of the value given.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| `service_provided` always carries an implementation | types: a provider returns a `Provided`, which always holds one, and `ServiceProvided::implementation` is not an `Option` |
| The implementation is the instance's concrete type, not the service type | `provide!` reads `type_name_of_val` of the value before it is boxed as the service type |
| A provider cannot name the implementation itself | `Implementation` has no public constructor taking text: only `__of`, which names the type of a value, and the crate's own `of_type`. `Provided` is made by `provide!`, through `__of` and `__new`, and by `from_box`, which names `T`, and it has no setter for the implementation. `__of` and `__new` are hidden and outside the API: calling them by hand could still pair the name of one value's type with another instance |
| Where the concrete type cannot be told, the implementation is the declared type | `from_box` names `T`, through `Implementation::of_type` |
| A provider MAY supply data with the instance, and the event carries none otherwise | `provide!(instance, data)`, `with_data`, and `provide!(instance)` or `from_box` alone; `ServiceProvided::data` is an `Option` |
| The data is a value, captured when the provider returns, and never serialized by the executor | types: `AnyValue::new` takes a `T: Value` by value; see the excluded scenarios |
| The data reaches reporters only, never the data bag, the result or the step | `engine/services.rs` splits what the provider returned with `into_parts`, emits the implementation and the data on `service_provided`, and keeps or lends only the instance |
| An instance of journey scope lent again causes no event | unchanged from 0095: only a call to the provider emits `service_provided` |
| `service_provider_failed` carries neither | unchanged: a provider that fails returned no `Provided` |
| The listing does not show implementations | unchanged: the listing knows only declarations |

## Tests

- Unit tests in `itinera-core/src/service/implementation.rs`: `a_value_is_the_implementation_its_concrete_type_names`.
- Unit tests in `itinera-core/src/service/provided.rs`: `the_implementation_is_the_concrete_type_of_the_instance_not_the_service_type`, `an_instance_already_boxed_is_named_by_the_type_it_is_boxed_as` and `an_erased_instance_is_still_lent_as_the_service_type`.
- Unit tests in `itinera-core/src/engine/services/tests.rs`: `service_provided_names_the_type_each_provider_made_and_carries_the_data_it_supplied`.
- Unit tests in `itinera-macros/src/workflow/parts/provider.rs`: `a_provider_makes_the_type_its_return_type_provides_in_a_result`, now reading `Result<Provided<T>, ...>`.
- The runner: two implementations of the cases' `labeller`, `FirstLabeller` and `SecondLabeller`, made as the scenario says with the data it gives, and the `implementation` column, which names an implementation by its label when Rust's name for it is the name of one of them. A provider of a `meter` given an implementation is a case error. Unit tests in `itinera-conformance/src/service.rs`, `model/service.rs`, `sentences/services.rs` and `trace/tests.rs`.
- UI tests in `crates/itinera/tests/ui/service/`: `a_provider_returning_a_bare_instance.rs`, a provider returning `Result<Warehouse, Error>`, does not compile.

## Excluded scenarios

| Scenario, in `implementation.feature` | Tag | Proof in `crates/itinera/tests/proofs.rs` |
|---|---|---|
| "A provider's data that is not a value aborts the journey, and no service event follows" | `@non-value` | `a_providers_data_cannot_be_a_non_value`: supplying a closure as a provider's data with `provide!` does not compile, since the data must be `Value`; the twin supplies an integer |

## Done when

Every scenario tagged `@proposal-0104` passes or is proven impossible to express, and 104 is listed in `conformance.json`.
