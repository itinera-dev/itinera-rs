# 0116: Duplicate adapter names, and a step named twice in one list

Tech spec for [proposal 0116](https://github.com/itinera-dev/spec/blob/main/proposals/0116-duplicate-adapter-names.md), implemented in [#176](https://github.com/itinera-dev/itinera-rs/issues/176). It amends [0060](0060-input-adapters-attached-to-steps.md) and [0095](0095-services-provided-to-steps.md). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done.

## API

- **`itinera::workflow::Violation::DuplicateAdapterName`** names the adapter whose name two or more adapters share, and **`ViolationKind::DuplicateAdapterName`** displays as `duplicate adapter name`.
- **`WorkflowBuilder::input_adapter`** no longer panics on a name the workflow already uses: `build()` reports the violation, with every other violation found.
- **`InputAdapterDescriptor::step`** and **`ServiceProviderDescriptor::step`** keep a step the adapter or provider is already attached to, rather than dropping it, so `build()` reports the repeated attachment. `Violation::StepAdaptedTwice` and `Violation::ServiceProvidedTwice` name the adapter or provider once for each attachment.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| Two input adapters with the same name are `duplicate adapter name`, whatever steps they are attached to | `build()`, in `workflow/violation/admission/adapters.rs` |
| An input adapter and a service provider may have the same name | admission counts adapter names and provider names apart, and the builder compares an adapter's name only with other adapters' |
| One adapter naming a step or rescue step twice is `step adapted twice` | `build()` groups every attachment by step, so one adapter attached twice counts twice |
| One provider naming a step or rescue step twice is `service provided twice` | `build()` groups every attachment by key and step, in `workflow/violation/admission/services.rs` |
| The same rules, in a workflow the macros declare | compile errors, in `itinera-macros/src/workflow/parts/attached.rs` for adapters and `itinera-macros/src/workflow/parts/provided.rs` for providers, whose messages name the violations |

## Tests

- Unit tests in `itinera-core/src/workflow/violation/admission/adapters.rs`: the cases `one_adapter_on_one_step_twice`, `two_adapters_with_one_name` and `one_adapter_on_one_rescue_step_twice` of `a_workflow_whose_input_adapters_are_declared_wrongly_is_refused_with_its_violation`.
- Unit tests in `itinera-core/src/workflow/violation/admission/services.rs`: the cases `one_provider_attached_to_one_step_twice` and `one_provider_attached_to_one_rescue_step_twice` of `a_workflow_whose_providers_are_declared_wrongly_is_refused_with_its_violation`, and `an_input_adapter_and_a_service_provider_may_share_a_name`.
- A unit test of the conformance runner, in `itinera-conformance/src/sentences/input_adapters.rs`: `a_second_input_adapter_with_a_used_name_declares_another_adapter`.
- The compile-fail tests `a_step_listed_twice_by_one_adapter`, `an_input_adapter_named_like_another` and `a_provider_listing_a_step_twice`, under `itinera/tests/ui`.
- The conformance cases tagged `@proposal-0116`.
