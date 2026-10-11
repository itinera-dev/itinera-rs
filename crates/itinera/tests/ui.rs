//! Rules that Rust's types enforce: each program breaking a rule fails to compile, and its twin,
//! which keeps the rule, compiles and runs.

fn enforced(broken: &str, kept: &str) {
    let cases = trybuild::TestCases::new();
    cases.compile_fail(broken);
    cases.pass(kept);
}

#[test]
fn an_instance_runs_at_most_one_journey() {
    enforced("tests/ui/run_twice.rs", "tests/ui/run_a_new_instance.rs");
}

#[test]
fn a_policy_attached_defines_at_least_one_hook() {
    enforced(
        "tests/ui/attach_a_hookless_policy.rs",
        "tests/ui/attach_a_hooked_policy.rs",
    );
}

#[test]
fn a_hook_requests_only_what_its_kind_may_request() {
    enforced(
        "tests/ui/request_the_reason_on_step_success.rs",
        "tests/ui/request_the_reason_on_step_failure.rs",
    );
}

#[test]
fn an_executor_runs_one_journey_at_a_time() {
    enforced(
        "tests/ui/run_concurrently.rs",
        "tests/ui/run_one_after_the_other.rs",
    );
}

/// What the step macros refuse, each with the macro's own error.
#[cfg(feature = "macros")]
mod step {
    use super::enforced;

    #[test]
    fn every_field_of_a_derived_step_is_an_input_or_a_default() {
        enforced(
            "tests/ui/step/unmarked_field.rs",
            "tests/ui/step/described.rs",
        );
    }

    #[test]
    fn only_an_optional_input_is_marked_required() {
        enforced(
            "tests/ui/step/required_on_a_required_input.rs",
            "tests/ui/step/built_by_its_new.rs",
        );
    }

    #[test]
    fn a_described_step_is_built_from_its_fields_or_by_its_new_never_both() {
        enforced(
            "tests/ui/step/derived_with_a_new.rs",
            "tests/ui/step/built_by_its_new.rs",
        );
    }

    #[test]
    fn a_described_step_is_built_from_its_fields_or_by_its_new() {
        enforced(
            "tests/ui/step/built_from_nothing.rs",
            "tests/ui/step/described.rs",
        );
    }

    #[test]
    fn a_step_block_holds_the_steps_run() {
        enforced("tests/ui/step/without_run.rs", "tests/ui/step/described.rs");
    }

    #[test]
    fn a_steps_run_takes_the_step_by_value() {
        enforced(
            "tests/ui/step/run_by_reference.rs",
            "tests/ui/step/described.rs",
        );
    }

    #[test]
    fn a_step_takes_each_handle_once() {
        enforced(
            "tests/ui/step/a_handle_taken_twice.rs",
            "tests/ui/step/described.rs",
        );
    }

    #[test]
    fn a_generic_step_is_described_with_the_builder() {
        enforced("tests/ui/step/generic.rs", "tests/ui/step/described.rs");
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_step_belongs_only_to_an_asynchronous_workflow() {
        enforced(
            "tests/ui/step/asynchronous_in_a_synchronous_workflow.rs",
            "tests/ui/step/asynchronous.rs",
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn an_asynchronous_step_needs_the_async_feature() {
        enforced(
            "tests/ui/step/asynchronous_without_the_async_feature.rs",
            "tests/ui/step/described.rs",
        );
    }

    #[test]
    fn every_field_of_a_derived_step_carries_one_mark() {
        enforced(
            "tests/ui/step/two_marks_on_a_field.rs",
            "tests/ui/step/described.rs",
        );
    }

    #[test]
    fn every_parameter_of_a_steps_new_is_an_input() {
        enforced(
            "tests/ui/step/unmarked_input_of_new.rs",
            "tests/ui/step/built_by_its_new.rs",
        );
    }

    #[test]
    fn every_parameter_of_a_steps_run_is_a_handle() {
        enforced(
            "tests/ui/step/unmarked_handle.rs",
            "tests/ui/step/described.rs",
        );
    }

    #[test]
    fn a_handle_has_the_type_its_mark_names() {
        enforced(
            "tests/ui/step/a_handle_of_the_wrong_type.rs",
            "tests/ui/step/described.rs",
        );
    }

    #[test]
    fn a_steps_new_that_may_fail_returns_a_result_written_as_one() {
        enforced(
            "tests/ui/step/a_new_returning_an_alias_of_result.rs",
            "tests/ui/step/built_by_a_new_that_may_fail.rs",
        );
    }

    #[test]
    fn a_step_is_named_where_a_workflow_lists_it() {
        enforced(
            "tests/ui/step/named_in_the_block.rs",
            "tests/ui/step/described.rs",
        );
    }

    #[test]
    fn an_input_takes_only_its_key_and_required() {
        enforced(
            "tests/ui/step/unknown_input_argument.rs",
            "tests/ui/step/described.rs",
        );
    }

    #[test]
    fn a_generic_step_does_not_derive_step() {
        enforced(
            "tests/ui/step/derived_generic.rs",
            "tests/ui/step/described.rs",
        );
    }
}

/// What the policy macros refuse, each with the macro's own error or the type that rules it out.
#[cfg(feature = "macros")]
mod policy {
    use super::enforced;

    #[test]
    fn a_hook_is_defined_once() {
        enforced(
            "tests/ui/policy/a_hook_defined_twice.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_step_policy_defines_only_step_hooks() {
        enforced(
            "tests/ui/policy/a_workflow_hook_in_a_step_policy.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_hook_returns_what_its_hook_returns() {
        enforced(
            "tests/ui/policy/a_hook_returning_the_wrong_type.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_hook_requests_only_what_its_hook_may_request() {
        enforced(
            "tests/ui/policy/a_request_its_hook_may_not_make.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_policy_is_attached_only_to_a_workflow_providing_its_roles() {
        enforced(
            "tests/ui/policy/a_role_the_workflow_does_not_provide.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn only_on_step_success_finishes_the_workflow() {
        enforced(
            "tests/ui/policy/finish_workflow_outside_on_step_success.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_policy_defines_at_least_one_hook() {
        enforced(
            "tests/ui/policy/without_hooks.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn every_parameter_of_a_hook_says_what_it_requests() {
        enforced(
            "tests/ui/policy/an_unmarked_request.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_policy_takes_only_its_name() {
        enforced(
            "tests/ui/policy/an_unknown_argument.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_generic_policy_is_described_with_the_builder() {
        enforced("tests/ui/policy/generic.rs", "tests/ui/policy/described.rs");
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_policy_belongs_only_to_an_asynchronous_workflow() {
        enforced(
            "tests/ui/policy/asynchronous_in_a_synchronous_workflow.rs",
            "tests/ui/policy/asynchronous.rs",
        );
    }

    #[cfg(feature = "async")]
    #[test]
    fn a_policys_hooks_are_all_synchronous_or_all_asynchronous() {
        enforced(
            "tests/ui/policy/synchronous_and_asynchronous_hooks.rs",
            "tests/ui/policy/asynchronous.rs",
        );
    }

    #[test]
    fn a_request_has_the_type_its_attribute_names() {
        enforced(
            "tests/ui/policy/a_request_of_the_wrong_type.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_policy_is_described_on_its_own_block() {
        enforced(
            "tests/ui/policy/on_a_trait_implementation.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_policy_has_one_factory() {
        enforced(
            "tests/ui/policy/two_factories.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_policys_factory_takes_no_parameters() {
        enforced(
            "tests/ui/policy/a_factory_with_parameters.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_policys_factory_is_synchronous() {
        enforced(
            "tests/ui/policy/an_asynchronous_factory.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_hook_takes_the_policy_by_reference() {
        enforced(
            "tests/ui/policy/a_hook_taking_the_policy_by_value.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_role_is_taken_by_shared_reference() {
        enforced(
            "tests/ui/policy/a_role_taken_by_value.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_function_is_one_hook_or_the_factory() {
        enforced(
            "tests/ui/policy/a_function_marked_twice.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn every_parameter_of_a_hook_requests_one_thing() {
        enforced(
            "tests/ui/policy/a_parameter_marked_twice.rs",
            "tests/ui/policy/described.rs",
        );
    }

    #[test]
    fn a_workflow_policy_defines_only_workflow_hooks() {
        enforced(
            "tests/ui/policy/a_step_hook_in_a_workflow_policy.rs",
            "tests/ui/policy/described_workflow_policy.rs",
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn an_asynchronous_hook_needs_the_async_feature() {
        enforced(
            "tests/ui/policy/asynchronous_without_the_async_feature.rs",
            "tests/ui/policy/described.rs",
        );
    }
}

/// What the journey data derives refuse, each with the derive's own error.
#[cfg(feature = "macros")]
mod journey {
    use super::enforced;

    #[test]
    fn only_a_key_whose_value_is_an_option_is_marked_required() {
        enforced(
            "tests/ui/journey/required_on_a_value_that_is_not_an_option.rs",
            "tests/ui/journey/described.rs",
        );
    }

    #[test]
    fn a_field_names_its_key_once() {
        enforced(
            "tests/ui/journey/a_key_named_twice.rs",
            "tests/ui/journey/described.rs",
        );
    }

    #[test]
    fn two_fields_never_give_the_same_key() {
        enforced(
            "tests/ui/journey/a_key_given_by_two_fields.rs",
            "tests/ui/journey/described.rs",
        );
    }

    #[test]
    fn required_takes_no_argument() {
        enforced(
            "tests/ui/journey/an_argument_to_required.rs",
            "tests/ui/journey/described.rs",
        );
    }

    #[test]
    fn a_key_is_named_as_a_string() {
        enforced(
            "tests/ui/journey/a_key_that_is_not_a_string.rs",
            "tests/ui/journey/described.rs",
        );
    }

    #[test]
    fn every_field_of_journey_data_is_a_value() {
        enforced(
            "tests/ui/journey/a_field_that_is_not_a_value.rs",
            "tests/ui/journey/described.rs",
        );
    }

    #[test]
    fn journey_data_is_a_struct_with_named_fields() {
        enforced(
            "tests/ui/journey/a_tuple_struct.rs",
            "tests/ui/journey/described.rs",
        );
    }

    #[test]
    fn generic_journey_data_implements_its_traits_by_hand() {
        enforced(
            "tests/ui/journey/generic.rs",
            "tests/ui/journey/described.rs",
        );
    }
}

/// What the workflow macros refuse, each with the macro's own error or the check it generates.
#[cfg(feature = "macros")]
mod workflow {
    use super::enforced;

    #[test]
    fn a_workflow_lists_its_steps_below_its_attribute() {
        enforced(
            "tests/ui/workflow/with_step_above_workflow.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_step_is_named_by_the_workflow_that_lists_it() {
        enforced(
            "tests/ui/workflow/a_step_without_a_name.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_workflow_names_each_step_once() {
        enforced(
            "tests/ui/workflow/a_duplicate_step_name.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_workflow_lists_at_least_one_step() {
        enforced(
            "tests/ui/workflow/no_step.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_workflow_takes_only_its_arguments() {
        enforced(
            "tests/ui/workflow/an_unknown_argument.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_step_listed_takes_only_its_settings() {
        enforced(
            "tests/ui/workflow/an_unknown_step_setting.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_generic_workflow_is_declared_with_the_builder() {
        enforced(
            "tests/ui/workflow/a_generic_workflow.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn an_instance_is_created_once_every_required_field_is_set() {
        enforced(
            "tests/ui/workflow/a_setter_missing_at_create.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn no_field_is_named_like_a_method_of_the_builder() {
        enforced(
            "tests/ui/workflow/a_field_named_like_a_builder_method.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn an_input_adapter_is_attached_only_to_steps_the_workflow_lists() {
        enforced(
            "tests/ui/workflow/an_input_adapter_for_an_unknown_step.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_step_is_adapted_once() {
        enforced(
            "tests/ui/workflow/a_step_adapted_twice.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn an_input_adapter_lists_each_step_once() {
        enforced(
            "tests/ui/workflow/a_step_listed_twice_by_one_adapter.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn an_input_adapter_is_attached_to_at_least_one_step() {
        enforced(
            "tests/ui/workflow/an_input_adapter_for_no_step.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn an_input_adapter_is_named_once() {
        enforced(
            "tests/ui/workflow/an_input_adapter_named_like_another.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_workflow_has_one_id_generator() {
        enforced(
            "tests/ui/workflow/two_id_generators.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_hook_is_defined_once_on_a_step() {
        enforced(
            "tests/ui/workflow/a_hook_defined_twice_on_a_step.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_hook_is_defined_once_on_the_workflow() {
        enforced(
            "tests/ui/workflow/a_hook_defined_twice_on_the_workflow.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_workflow_provides_the_roles_its_policies_request() {
        enforced(
            "tests/ui/workflow/a_role_the_workflow_does_not_provide.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_role_is_provided_from_an_implementation_of_it() {
        enforced(
            "tests/ui/workflow/a_role_on_the_workflows_own_block.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn a_workflows_fields_have_names() {
        enforced(
            "tests/ui/workflow/a_tuple_struct.rs",
            "tests/ui/workflow/described_courier.rs",
        );
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_reporter_is_listed_among_the_asynchronous_reporters() {
        enforced(
            "tests/ui/workflow/an_asynchronous_reporter_listed_as_a_reporter.rs",
            "tests/ui/workflow/described_asynchronous.rs",
        );
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_policy_belongs_only_to_an_asynchronous_workflow() {
        enforced(
            "tests/ui/workflow/an_asynchronous_policy_in_a_synchronous_workflow.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn asynchronous_reporters_belong_to_an_asynchronous_workflow() {
        enforced(
            "tests/ui/workflow/asynchronous_reporters_in_a_synchronous_workflow.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[test]
    fn the_parts_of_a_workflow_belong_to_a_declared_workflow() {
        enforced(
            "tests/ui/workflow/parts_of_a_type_that_is_not_a_workflow.rs",
            "tests/ui/workflow/described.rs",
        );
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_step_belongs_only_to_an_asynchronous_workflow() {
        enforced(
            "tests/ui/workflow/an_asynchronous_step_in_a_synchronous_workflow.rs",
            "tests/ui/workflow/described_asynchronous.rs",
        );
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn an_asynchronous_workflow_needs_the_async_feature() {
        enforced(
            "tests/ui/workflow/asynchronous_without_the_async_feature.rs",
            "tests/ui/workflow/described.rs",
        );
    }
}
