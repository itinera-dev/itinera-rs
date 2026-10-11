//! Rules that Rust's types enforce: each program breaking a rule fails to compile, and its twin,
//! which keeps the rule, compiles, and runs when it has something to run.

use std::error::Error;

#[path = "ui/attach_a_hooked_policy.rs"]
mod attach_a_hooked_policy;
#[path = "ui/request_the_reason_on_step_failure.rs"]
#[expect(dead_code, reason = "this twin only has to compile")]
mod request_the_reason_on_step_failure;
#[path = "ui/run_a_new_instance.rs"]
mod run_a_new_instance;
#[path = "ui/run_one_after_the_other.rs"]
mod run_one_after_the_other;

fn fails_to_compile(broken: &str) {
    trybuild::TestCases::new().compile_fail(broken);
}

#[test]
fn an_instance_runs_at_most_one_journey() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/ui/run_twice.rs");
    run_a_new_instance::main()
}

#[test]
fn a_policy_attached_defines_at_least_one_hook() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/ui/attach_a_hookless_policy.rs");
    attach_a_hooked_policy::main()
}

#[test]
fn a_hook_requests_only_what_its_kind_may_request() {
    fails_to_compile("tests/ui/request_the_reason_on_step_success.rs");
}

#[test]
fn an_executor_runs_one_journey_at_a_time() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/ui/run_concurrently.rs");
    run_one_after_the_other::main()
}

/// What the step macros refuse, each with the macro's own error.
#[cfg(feature = "macros")]
#[path = "ui/step"]
mod step {
    use super::fails_to_compile;
    use std::error::Error;

    #[cfg(feature = "async")]
    mod asynchronous;
    mod built_by_a_new_that_may_fail;
    mod built_by_its_new;
    pub(super) mod described;

    #[test]
    fn every_field_of_a_derived_step_is_an_input_or_a_default() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/unmarked_field.rs");
        described::main()
    }

    #[test]
    fn only_an_optional_input_is_marked_required() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/required_on_a_required_input.rs");
        built_by_its_new::main()
    }

    #[test]
    fn a_described_step_is_built_from_its_fields_or_by_its_new_never_both()
    -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/derived_with_a_new.rs");
        built_by_its_new::main()
    }

    #[test]
    fn a_described_step_is_built_from_its_fields_or_by_its_new() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/built_from_nothing.rs");
        described::main()
    }

    #[test]
    fn a_step_block_holds_the_steps_run() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/without_run.rs");
        described::main()
    }

    #[test]
    fn a_steps_run_takes_the_step_by_value() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/run_by_reference.rs");
        described::main()
    }

    #[test]
    fn a_step_takes_each_handle_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/a_handle_taken_twice.rs");
        described::main()
    }

    #[test]
    fn a_generic_step_is_described_with_the_builder() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/generic.rs");
        described::main()
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_step_belongs_only_to_an_asynchronous_workflow() -> Result<(), Box<dyn Error>>
    {
        fails_to_compile("tests/ui/step/asynchronous_in_a_synchronous_workflow.rs");
        asynchronous::main()
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn an_asynchronous_step_needs_the_async_feature() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/asynchronous_without_the_async_feature.rs");
        described::main()
    }

    #[test]
    fn every_field_of_a_derived_step_carries_one_mark() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/two_marks_on_a_field.rs");
        described::main()
    }

    #[test]
    fn every_parameter_of_a_steps_new_is_an_input() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/unmarked_input_of_new.rs");
        built_by_its_new::main()
    }

    #[test]
    fn every_parameter_of_a_steps_run_is_a_handle() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/unmarked_handle.rs");
        described::main()
    }

    #[test]
    fn a_handle_has_the_type_its_mark_names() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/a_handle_of_the_wrong_type.rs");
        described::main()
    }

    #[test]
    fn a_steps_new_that_may_fail_returns_a_result_written_as_one() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/a_new_returning_an_alias_of_result.rs");
        built_by_a_new_that_may_fail::main()
    }

    #[test]
    fn a_step_is_named_where_a_workflow_lists_it() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/named_in_the_block.rs");
        described::main()
    }

    #[test]
    fn an_input_takes_only_its_key_and_required() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/unknown_input_argument.rs");
        described::main()
    }

    #[test]
    fn a_generic_step_does_not_derive_step() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/step/derived_generic.rs");
        described::main()
    }
}

/// What the rescue step macros refuse, each with the macro's own error or the type that rules it
/// out.
#[cfg(feature = "macros")]
#[path = "ui/rescue"]
mod rescue {
    use super::fails_to_compile;
    use std::error::Error;

    #[cfg(feature = "async")]
    mod asynchronous;
    pub(super) mod described;

    #[test]
    fn a_rescue_step_never_skips() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/rescue/skipping.rs");
        described::main()
    }

    #[test]
    fn every_field_of_a_derived_rescue_step_is_a_request() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/rescue/unmarked_field.rs");
        described::main()
    }

    #[test]
    fn the_reason_and_the_error_are_requested_without_a_key() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/rescue/a_reason_with_a_key.rs");
        described::main()
    }

    #[test]
    fn a_rescue_step_block_holds_the_rescue_steps_run() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/rescue/without_run.rs");
        described::main()
    }

    #[test]
    fn a_rescue_step_is_never_named() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/rescue/named.rs");
        described::main()
    }

    #[test]
    fn a_generic_rescue_step_is_given_with_the_builder() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/rescue/generic.rs");
        described::main()
    }

    #[test]
    fn a_generic_rescue_step_does_not_derive_rescue_step() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/rescue/derived_generic.rs");
        described::main()
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_rescue_step_belongs_only_to_an_asynchronous_workflow()
    -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/rescue/asynchronous_in_a_synchronous_workflow.rs");
        described::main()
    }

    #[cfg(feature = "async")]
    #[test]
    fn a_synchronous_rescue_step_belongs_only_to_a_synchronous_workflow()
    -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/rescue/synchronous_in_an_asynchronous_workflow.rs");
        asynchronous::main()
    }
}

/// What the policy macros refuse, each with the macro's own error or the type that rules it out.
#[cfg(feature = "macros")]
#[path = "ui/policy"]
mod policy {
    use super::fails_to_compile;
    use std::error::Error;

    #[cfg(feature = "async")]
    mod asynchronous;
    mod described;
    mod described_workflow_policy;

    #[test]
    fn a_hook_is_defined_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_hook_defined_twice.rs");
        described::main()
    }

    #[test]
    fn a_step_policy_defines_only_step_hooks() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_workflow_hook_in_a_step_policy.rs");
        described::main()
    }

    #[test]
    fn a_hook_returns_what_its_hook_returns() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_hook_returning_the_wrong_type.rs");
        described::main()
    }

    #[test]
    fn a_hook_requests_only_what_its_hook_may_request() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_request_its_hook_may_not_make.rs");
        described::main()
    }

    #[test]
    fn a_policy_is_attached_only_to_a_workflow_providing_its_roles() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_role_the_workflow_does_not_provide.rs");
        described::main()
    }

    #[test]
    fn only_on_step_success_finishes_the_workflow() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/finish_workflow_outside_on_step_success.rs");
        described::main()
    }

    #[test]
    fn a_policy_defines_at_least_one_hook() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/without_hooks.rs");
        described::main()
    }

    #[test]
    fn every_parameter_of_a_hook_says_what_it_requests() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/an_unmarked_request.rs");
        described::main()
    }

    #[test]
    fn a_policy_takes_only_its_name() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/an_unknown_argument.rs");
        described::main()
    }

    #[test]
    fn a_generic_policy_is_described_with_the_builder() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/generic.rs");
        described::main()
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_policy_belongs_only_to_an_asynchronous_workflow()
    -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/asynchronous_in_a_synchronous_workflow.rs");
        asynchronous::main()
    }

    #[cfg(feature = "async")]
    #[test]
    fn a_policys_hooks_are_all_synchronous_or_all_asynchronous() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/synchronous_and_asynchronous_hooks.rs");
        asynchronous::main()
    }

    #[test]
    fn a_request_has_the_type_its_attribute_names() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_request_of_the_wrong_type.rs");
        described::main()
    }

    #[test]
    fn a_policy_is_described_on_its_own_block() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/on_a_trait_implementation.rs");
        described::main()
    }

    #[test]
    fn a_policy_has_one_factory() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/two_factories.rs");
        described::main()
    }

    #[test]
    fn a_policys_factory_takes_no_parameters() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_factory_with_parameters.rs");
        described::main()
    }

    #[test]
    fn a_policys_factory_is_synchronous() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/an_asynchronous_factory.rs");
        described::main()
    }

    #[test]
    fn a_hook_takes_the_policy_by_reference() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_hook_taking_the_policy_by_value.rs");
        described::main()
    }

    #[test]
    fn a_role_is_taken_by_shared_reference() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_role_taken_by_value.rs");
        described::main()
    }

    #[test]
    fn a_function_is_one_hook_or_the_factory() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_function_marked_twice.rs");
        described::main()
    }

    #[test]
    fn every_parameter_of_a_hook_requests_one_thing() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_parameter_marked_twice.rs");
        described::main()
    }

    #[test]
    fn a_workflow_policy_defines_only_workflow_hooks() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/a_step_hook_in_a_workflow_policy.rs");
        described_workflow_policy::main()
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn an_asynchronous_hook_needs_the_async_feature() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/policy/asynchronous_without_the_async_feature.rs");
        described::main()
    }
}

/// What the journey data derives refuse, each with the derive's own error.
#[cfg(feature = "macros")]
#[path = "ui/journey"]
mod journey {
    use super::fails_to_compile;
    use std::error::Error;

    mod described;

    #[test]
    fn only_a_key_whose_value_is_an_option_is_marked_required() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/journey/required_on_a_value_that_is_not_an_option.rs");
        described::main()
    }

    #[test]
    fn a_field_names_its_key_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/journey/a_key_named_twice.rs");
        described::main()
    }

    #[test]
    fn two_fields_never_give_the_same_key() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/journey/a_key_given_by_two_fields.rs");
        described::main()
    }

    #[test]
    fn required_takes_no_argument() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/journey/an_argument_to_required.rs");
        described::main()
    }

    #[test]
    fn a_key_is_named_as_a_string() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/journey/a_key_that_is_not_a_string.rs");
        described::main()
    }

    #[test]
    fn every_field_of_journey_data_is_a_value() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/journey/a_field_that_is_not_a_value.rs");
        described::main()
    }

    #[test]
    fn journey_data_is_a_struct_with_named_fields() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/journey/a_tuple_struct.rs");
        described::main()
    }

    #[test]
    fn generic_journey_data_implements_its_traits_by_hand() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/journey/generic.rs");
        described::main()
    }
}

/// What the workflow macros refuse, each with the macro's own error or the check it generates.
#[cfg(feature = "macros")]
#[path = "ui/workflow"]
mod workflow {
    use super::fails_to_compile;
    use std::error::Error;

    pub(super) mod described;
    #[cfg(feature = "async")]
    mod described_asynchronous;
    mod described_courier;

    #[test]
    fn a_workflow_lists_its_steps_below_its_attribute() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/with_step_above_workflow.rs");
        described::main()
    }

    #[test]
    fn a_step_is_named_by_the_workflow_that_lists_it() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_step_without_a_name.rs");
        described::main()
    }

    #[test]
    fn a_workflow_names_each_step_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_duplicate_step_name.rs");
        described::main()
    }

    #[test]
    fn a_step_with_a_rescue_step_has_no_retry_budget() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_rescued_step_with_a_retry_budget.rs");
        super::rescue::described::main()
    }

    #[test]
    fn no_input_adapter_is_attached_to_a_rescue_step() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/an_input_adapter_for_a_rescue_step.rs");
        super::rescue::described::main()
    }

    #[test]
    fn no_step_is_named_like_a_rescue_step() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_step_named_like_a_rescue_step.rs");
        super::rescue::described::main()
    }

    #[test]
    fn a_workflow_lists_at_least_one_step() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/no_step.rs");
        described::main()
    }

    #[test]
    fn a_workflow_takes_only_its_arguments() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/an_unknown_argument.rs");
        described::main()
    }

    #[test]
    fn a_step_listed_takes_only_its_settings() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/an_unknown_step_setting.rs");
        described::main()
    }

    #[test]
    fn a_generic_workflow_is_declared_with_the_builder() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_generic_workflow.rs");
        described::main()
    }

    #[test]
    fn an_instance_is_created_once_every_required_field_is_set() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_setter_missing_at_create.rs");
        described::main()
    }

    #[test]
    fn no_field_is_named_like_a_method_of_the_builder() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_field_named_like_a_builder_method.rs");
        described::main()
    }

    #[test]
    fn an_input_adapter_is_attached_only_to_steps_the_workflow_lists() -> Result<(), Box<dyn Error>>
    {
        fails_to_compile("tests/ui/workflow/an_input_adapter_for_an_unknown_step.rs");
        described::main()
    }

    #[test]
    fn a_step_is_adapted_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_step_adapted_twice.rs");
        described::main()
    }

    #[test]
    fn an_input_adapter_lists_each_step_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_step_listed_twice_by_one_adapter.rs");
        described::main()
    }

    #[test]
    fn an_input_adapter_is_attached_to_at_least_one_step() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/an_input_adapter_for_no_step.rs");
        described::main()
    }

    #[test]
    fn an_input_adapter_is_named_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/an_input_adapter_named_like_another.rs");
        described::main()
    }

    #[test]
    fn a_workflow_has_one_id_generator() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/two_id_generators.rs");
        described::main()
    }

    #[test]
    fn a_hook_is_defined_once_on_a_step() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_hook_defined_twice_on_a_step.rs");
        described::main()
    }

    #[test]
    fn a_hook_is_defined_once_on_the_workflow() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_hook_defined_twice_on_the_workflow.rs");
        described::main()
    }

    #[test]
    fn a_workflow_provides_the_roles_its_policies_request() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_role_the_workflow_does_not_provide.rs");
        described::main()
    }

    #[test]
    fn a_role_is_provided_from_an_implementation_of_it() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_role_on_the_workflows_own_block.rs");
        described::main()
    }

    #[test]
    fn a_workflows_fields_have_names() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/a_tuple_struct.rs");
        described_courier::main()
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_reporter_is_listed_among_the_asynchronous_reporters()
    -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/an_asynchronous_reporter_listed_as_a_reporter.rs");
        described_asynchronous::main()
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_policy_belongs_only_to_an_asynchronous_workflow()
    -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/an_asynchronous_policy_in_a_synchronous_workflow.rs");
        described::main()
    }

    #[test]
    fn asynchronous_reporters_belong_to_an_asynchronous_workflow() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/asynchronous_reporters_in_a_synchronous_workflow.rs");
        described::main()
    }

    #[test]
    fn the_parts_of_a_workflow_belong_to_a_declared_workflow() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/parts_of_a_type_that_is_not_a_workflow.rs");
        described::main()
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_step_belongs_only_to_an_asynchronous_workflow() -> Result<(), Box<dyn Error>>
    {
        fails_to_compile("tests/ui/workflow/an_asynchronous_step_in_a_synchronous_workflow.rs");
        described_asynchronous::main()
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn an_asynchronous_workflow_needs_the_async_feature() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/workflow/asynchronous_without_the_async_feature.rs");
        described::main()
    }
}

/// What the macros refuse in the services steps request and workflows provide, each with the
/// macro's own error or the check it generates.
#[cfg(all(feature = "macros", feature = "unstable"))]
#[path = "ui/service"]
mod service {
    use super::fails_to_compile;
    use std::error::Error;

    mod described;
    #[cfg(feature = "async")]
    mod described_asynchronous;
    mod described_with_a_rescue_step;
    mod described_with_providers_written_apart;

    #[test]
    fn a_step_requests_each_service_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_requested_twice.rs");
        described::main()
    }

    #[test]
    fn a_service_is_taken_by_shared_reference() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_taken_by_value.rs");
        described::main()
    }

    #[test]
    fn a_service_new_and_run_both_take_has_one_type() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_requested_as_two_types.rs");
        described::main()
    }

    #[test]
    fn a_service_is_named_by_a_string() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_key_that_is_not_a_string.rs");
        described::main()
    }

    #[test]
    fn a_workflow_provides_every_service_its_steps_request() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_not_provided.rs");
        described::main()
    }

    #[test]
    fn a_workflow_has_a_provider_for_every_key_its_steps_request() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_of_a_key_no_provider_supplies.rs");
        described::main()
    }

    #[test]
    fn a_step_requests_a_service_as_the_type_its_provider_makes() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_of_another_type_than_its_provider_makes.rs");
        described::main()
    }

    #[test]
    fn a_step_requests_a_service_as_the_type_its_attached_provider_makes()
    -> Result<(), Box<dyn Error>> {
        fails_to_compile(
            "tests/ui/service/a_service_of_another_type_than_its_attached_provider_makes.rs",
        );
        described::main()
    }

    #[test]
    fn providers_of_one_key_boxing_different_types_leave_it_to_the_descriptor()
    -> Result<(), Box<dyn Error>> {
        described_with_providers_written_apart::main()
    }

    #[test]
    fn a_rescue_step_is_served_only_by_a_provider_attached_to_no_step() -> Result<(), Box<dyn Error>>
    {
        fails_to_compile(
            "tests/ui/service/a_rescue_steps_service_only_an_attached_provider_supplies.rs",
        );
        described_with_a_rescue_step::main()
    }

    #[test]
    fn a_rescue_step_requests_a_service_as_the_type_its_provider_makes()
    -> Result<(), Box<dyn Error>> {
        fails_to_compile(
            "tests/ui/service/a_rescue_steps_service_of_another_type_than_its_provider_makes.rs",
        );
        described_with_a_rescue_step::main()
    }

    #[test]
    fn a_rescue_step_requests_each_service_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_requested_twice_by_a_rescue_step.rs");
        described_with_a_rescue_step::main()
    }

    #[test]
    fn a_provider_is_never_attached_to_a_rescue_step() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_provider_for_a_rescue_step.rs");
        described_with_a_rescue_step::main()
    }

    #[test]
    fn one_provider_attached_to_no_step_provides_a_service() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_provided_twice.rs");
        described::main()
    }

    #[test]
    fn one_provider_attached_to_a_step_provides_it_a_service() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_provided_twice_to_a_step.rs");
        described::main()
    }

    #[test]
    fn a_provider_is_named_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_provider_named_like_another.rs");
        described::main()
    }

    #[test]
    fn a_provider_is_attached_only_to_steps_the_workflow_lists() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_provider_for_an_unknown_step.rs");
        described::main()
    }

    #[test]
    fn a_provider_lists_each_step_once() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_provider_listing_a_step_twice.rs");
        described::main()
    }

    #[test]
    fn a_provider_attached_to_steps_lists_at_least_one() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_provider_for_no_step.rs");
        described::main()
    }

    #[test]
    fn only_a_provider_of_attempt_scope_requests_the_attempt() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_journey_scoped_provider_requesting_the_attempt.rs");
        described::main()
    }

    #[test]
    fn a_provider_takes_only_its_settings() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/an_unknown_provider_argument.rs");
        described::main()
    }

    #[test]
    fn a_provider_takes_the_workflow_by_reference() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_provider_taking_the_workflow_by_value.rs");
        described::main()
    }

    #[test]
    fn a_provider_returns_the_instance_it_makes() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_provider_without_a_return_type.rs");
        described::main()
    }

    #[test]
    fn a_provider_returns_its_instance_in_a_box() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_provider_of_a_type_that_is_not_a_box.rs");
        described::main()
    }

    #[test]
    fn every_parameter_of_a_provider_says_what_it_requests() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/an_unmarked_provider_request.rs");
        described::main()
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_provider_belongs_only_to_an_asynchronous_workflow()
    -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/an_asynchronous_provider_in_a_synchronous_workflow.rs");
        described_asynchronous::main()
    }

    #[cfg(feature = "async")]
    #[test]
    fn a_synchronous_provider_requesting_a_reporter_belongs_only_to_a_synchronous_workflow()
    -> Result<(), Box<dyn Error>> {
        fails_to_compile(
            "tests/ui/service/a_synchronous_provider_reporting_in_an_asynchronous_workflow.rs",
        );
        described_asynchronous::main()
    }
}

/// Services, which only the `unstable` feature offers.
#[cfg(all(feature = "macros", not(feature = "unstable")))]
mod without_services {
    use super::fails_to_compile;
    use std::error::Error;

    #[test]
    fn a_service_needs_the_unstable_feature() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_service_without_the_unstable_feature.rs");
        super::step::described::main()
    }

    #[test]
    fn a_service_provider_needs_the_unstable_feature() -> Result<(), Box<dyn Error>> {
        fails_to_compile("tests/ui/service/a_provider_without_the_unstable_feature.rs");
        super::workflow::described::main()
    }
}
