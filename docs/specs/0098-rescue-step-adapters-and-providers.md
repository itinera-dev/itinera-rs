# 0098: Input adapters and service providers for rescue steps

Tech spec for [proposal 0098](https://github.com/itinera-dev/spec/blob/main/proposals/0098-rescue-step-adapters-and-providers.md), implemented in [#116](https://github.com/itinera-dev/itinera-rs/issues/116). It amends [0003](0003-rescue-steps.md), [0060](0060-input-adapters-attached-to-steps.md) and [0095](0095-services-provided-to-steps.md). The [tier 1 plan](../tier-1-plan.md) holds what crosses proposals.

Status: done.

## API

- **Attaching to a rescue step** needs no new API. An input adapter or a service provider is attached to a rescue step by its name, the step's followed by `_rescue`, as to a step: `InputAdapterDescriptor::new("credentials", step_name!("charge_rescue"), adapt)` or `.step(step_name!("charge_rescue"))` with the builder, and `steps = ["charge_rescue"]` on `#[input_adapter]` or `#[provider]` with the macros.
- **The listing** gives a step with a rescue step a rescue section, `ListedStep::rescue`, an `Option<itinera::workflow::ListedRescue>`. `ListedRescue` holds the rescue step's name, `step`, the name of its input adapter, `adapter`, and the services it requests with the provider serving each, `services`, as `ListedStep` holds them for the step. It has no position and no policies.

## How the rules are enforced

| Rule | Enforced by |
|---|---|
| A rescue step's name may be named wherever an adapter or provider is attached to steps, only when the step has a rescue step | admission, in `workflow/violation/admission.rs`, counts the names of the steps' rescue steps among the names an adapter or provider may be attached to; any other name is `input adapter for unknown step` or `service provider for unknown step`. `#[workflow]` lists the rescue steps' names after the steps' in the constant its checks read, so a workflow the macros declare with either mistake does not compile |
| A rescue step has at most one input adapter, and one provider of each key attached to it | admission groups the attachments to a rescue step's name as it groups those to a step's, so two are `step adapted twice` or `service provided twice`; the macros compare the names written in `steps = [...]`, whatever they name |
| The rescue step's inputs are resolved through the adapter attached to its name, then from the data bag, with the events and aborts of any adapted step, naming the rescue step | the engine looks up the adapter of every step it builds by the step's name, in `engine/attempt.rs`, and the rescue step is built under its own name |
| An adapter attached to `<step>_rescue` that requests the step's name receives `<step>_rescue` | the same: the adapter is told the name of the step being built |
| Data the rescue step requests from the step it rescues is never asked of its adapter | the engine asks the adapter only for `StepRequest::Input`, never for a `StepRequest::Rescued` request |
| A rescue step is served by the provider of each key attached to its name, or else by the one attached to no step, never by one attached to the step it rescues | `serving`, in `service/descriptor/entry.rs`, serves every step and rescue step by its name, and admission checks the rescue step's services with it, in `services/requests.rs`. The macros' check of each rescue step's services names the rescue step, in `itinera-macros/src/workflow/services.rs` |
| A journey-scoped provider attached to a step and to its rescue step lends both one instance | the engine keeps one slot per journey-scoped provider, whichever step it serves |
| A provider of attempt scope serving a rescue step receives the rescue step's name and attempt 1 | unchanged from 0095 |
| The listing shows a rescue section for each step with a rescue step, and none otherwise | `listing()`, through `ListedStep::rescue` |
| Nothing is inherited from the step a rescue step rescues | an attachment names one step; nothing looks up the rescued step's attachments for its rescue step |

## Tests

- Unit tests in `itinera-core/src/workflow/violation/admission/adapters.rs`: the cases `two_adapters_on_one_rescue_step` and `an_adapter_on_the_rescue_step_of_a_step_without_one` of `a_workflow_whose_input_adapters_are_declared_wrongly_is_refused_with_its_violation`, and `an_adapter_may_be_attached_to_a_step_and_to_its_rescue_step`.
- Unit tests in `itinera-core/src/workflow/violation/admission/services.rs`: the cases `a_provider_attached_to_the_rescue_step_of_a_step_without_one` and `two_providers_attached_to_one_rescue_step` of `a_workflow_whose_providers_are_declared_wrongly_is_refused_with_its_violation`, and `a_provider_attached_to_a_rescue_steps_name_serves_it`; in `services/requests.rs`, the case `a_provider_attached_to_a_rescue_step_of_another_type_than_it_requests`.
- Unit tests in `itinera-core/src/service/descriptor/entry.rs`: the cases `the_rescue_step_it_is_attached_to` and `a_rescue_step_whose_only_provider_is_attached_to_its_step` of `a_provider_attached_to_a_step_serves_it_and_the_one_attached_to_no_step_serves_the_rest`.
- Unit tests in `itinera-core/src/engine/rescue/tests.rs`: `a_rescue_steps_inputs_come_only_from_the_adapter_attached_to_its_name_then_the_data_bag`; in `itinera-core/src/engine/services/tests.rs`, module `rescue_steps`: `a_provider_attached_to_a_rescue_steps_name_serves_it_instead_of_the_one_attached_to_no_step` and `a_journey_scoped_provider_attached_to_a_step_and_its_rescue_step_lends_both_one_instance`.
- Unit tests in `itinera-core/src/workflow/listing.rs`: `a_workflow_is_listed_in_order_without_running_anything`, whose rescue step has an input adapter, and `a_rescue_section_lists_the_services_its_rescue_step_requests_and_the_provider_serving_each`.
- An equivalence test in `crates/itinera/tests/macros/rescue_service.rs`: `a_rescue_steps_adapter_and_provider_the_macros_attach_to_its_name_list_and_run_as_with_the_builder`.
- UI tests in `crates/itinera/tests/ui.rs`: `an_input_adapter_is_attached_only_to_rescue_steps_the_workflow_has`, against `ui/rescue/adapted.rs`, which runs a rescue step whose input an adapter attached to its name supplies, and `a_provider_is_attached_only_to_rescue_steps_the_workflow_has`, against `ui/service/described_with_a_provider_for_a_rescue_step.rs`, which runs a rescue step served by a provider attached to it and to its step.
- The runner reads the listing's optional columns `rescue adapter` and `rescue services`, in `sentences/admission/listed.rs`, and the sentence for an adapter that returns the step name it receives, in `sentences/input_adapters.rs`.

## Done when

Every scenario tagged `@proposal-0098` passes, and 98 is listed in `conformance.json`.
