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
