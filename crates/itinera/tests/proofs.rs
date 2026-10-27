//! Proofs that rules the specification lets a language make impossible to express cannot be
//! written with itinera: each program breaking the rule fails to compile, and its twin, which
//! keeps the rule, compiles.

use std::error::Error;

#[cfg(feature = "async")]
#[path = "proofs/mode-not-accepted/asynchronous_provider_async_workflow.rs"]
mod asynchronous_provider_async_workflow;
#[cfg(feature = "async")]
#[path = "proofs/mode-not-accepted/asynchronous_step_async_executor.rs"]
mod asynchronous_step_async_executor;
#[path = "proofs/attempt-negative/attempt_counted_from_one.rs"]
mod attempt_counted_from_one;
#[path = "proofs/journey-scope-request/attempt_scoped_provider_asks_for_the_attempt.rs"]
mod attempt_scoped_provider_asks_for_the_attempt;
#[path = "proofs/non-value/contribution_value.rs"]
#[expect(dead_code, reason = "this twin only has to compile")]
mod contribution_value;
#[path = "proofs/non-value/event_data_value.rs"]
#[expect(dead_code, reason = "this twin only has to compile")]
mod event_data_value;
#[path = "proofs/invalid-lifecycle/fail_workflow_on_step_failure.rs"]
mod fail_workflow_on_step_failure;
#[path = "proofs/non-value/global_metadata_value.rs"]
#[expect(dead_code, reason = "this twin only has to compile")]
mod global_metadata_value;
#[path = "proofs/non-string-journey-id/id_generator_returning_a_string.rs"]
mod id_generator_returning_a_string;
#[path = "proofs/non-value/initial_data_value.rs"]
mod initial_data_value;
#[path = "proofs/non-value/initial_global_metadata_value.rs"]
mod initial_global_metadata_value;
#[path = "proofs/non-value/input_adapter_value.rs"]
mod input_adapter_value;
#[path = "proofs/non-value/provider_data_value.rs"]
mod provider_data_value;
#[path = "proofs/non-value/provider_event_data_value.rs"]
mod provider_event_data_value;
#[path = "proofs/no-instance/provider_returning_an_instance.rs"]
mod provider_returning_an_instance;
#[path = "proofs/late-handle/read_by_an_input_adapter_during_its_call.rs"]
mod read_by_an_input_adapter_during_its_call;
#[path = "proofs/bag-write/read_through_the_access.rs"]
mod read_through_the_access;
#[path = "proofs/non-value/reason_details_value.rs"]
mod reason_details_value;
#[path = "proofs/role-not-provided/role_provided.rs"]
mod role_provided;
#[path = "proofs/unexpected-result/step_factory_building_a_step.rs"]
mod step_factory_building_a_step;
#[path = "proofs/unexpected-result/step_policy_building_a_policy.rs"]
mod step_policy_building_a_policy;
#[path = "proofs/unexpected-result/step_returning_an_outcome.rs"]
mod step_returning_an_outcome;
#[path = "proofs/rescue-skip/succeeding_rescue_step.rs"]
mod succeeding_rescue_step;
#[path = "proofs/late-handle/used_by_a_thread_during_the_attempt.rs"]
#[expect(dead_code, reason = "this twin only has to compile")]
mod used_by_a_thread_during_the_attempt;
#[path = "proofs/unexpected-result/workflow_policy_building_a_policy.rs"]
mod workflow_policy_building_a_policy;

fn fails_to_compile(broken: &str) {
    trybuild::TestCases::new().compile_fail(broken);
}

#[test]
fn a_step_cannot_contribute_a_non_value() {
    fails_to_compile("tests/proofs/non-value/contribution.rs");
}

#[test]
fn initial_data_cannot_hold_a_non_value() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/non-value/initial_data.rs");
    initial_data_value::main()
}

#[test]
fn an_input_adapter_cannot_supply_a_non_value() {
    fails_to_compile("tests/proofs/non-value/input_adapter.rs");
    input_adapter_value::main();
}

#[test]
fn a_reasons_details_cannot_be_a_non_value() {
    fails_to_compile("tests/proofs/non-value/reason_details.rs");
    reason_details_value::main();
}

#[test]
fn a_step_event_cannot_carry_a_non_value() {
    fails_to_compile("tests/proofs/non-value/event_data.rs");
}

#[test]
fn a_step_cannot_set_a_non_value_as_global_metadata() {
    fails_to_compile("tests/proofs/non-value/global_metadata.rs");
}

#[test]
fn initial_global_metadata_cannot_hold_a_non_value() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/non-value/initial_global_metadata.rs");
    initial_global_metadata_value::main()
}

#[test]
fn a_failure_hook_cannot_finish_the_workflow() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/invalid-lifecycle/finish_workflow_on_step_failure.rs");
    fail_workflow_on_step_failure::main()
}

#[test]
fn a_rescue_step_cannot_skip() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/rescue-skip/skipping_rescue_step.rs");
    succeeding_rescue_step::main()
}

#[test]
fn a_policy_cannot_be_attached_to_a_workflow_that_does_not_provide_its_role()
-> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/role-not-provided/role_not_provided.rs");
    role_provided::main()
}

#[test]
#[cfg(feature = "async")]
fn a_synchronous_executor_cannot_run_an_asynchronous_step() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/mode-not-accepted/asynchronous_step.rs");
    asynchronous_step_async_executor::main()
}

#[test]
fn a_steps_handles_cannot_outlive_its_attempt() {
    fails_to_compile("tests/proofs/late-handle/kept_by_the_factory.rs");
    fails_to_compile("tests/proofs/late-handle/moved_into_a_thread.rs");
}

#[test]
fn an_adapters_access_to_the_data_bag_cannot_outlive_its_call() {
    fails_to_compile("tests/proofs/late-handle/kept_by_an_input_adapter.rs");
    fails_to_compile("tests/proofs/late-handle/kept_by_an_input_adapter_taking_a_static_call.rs");
    read_by_an_input_adapter_during_its_call::main();
}

#[test]
fn an_adapter_cannot_write_through_its_access_to_the_data_bag() {
    fails_to_compile("tests/proofs/bag-write/write_through_the_access.rs");
    read_through_the_access::main();
}

#[test]
fn a_journey_scoped_provider_cannot_request_the_attempt() {
    fails_to_compile(
        "tests/proofs/journey-scope-request/journey_scoped_provider_asks_for_the_attempt.rs",
    );
    attempt_scoped_provider_asks_for_the_attempt::main();
}

#[test]
fn a_provider_cannot_return_nothing() {
    fails_to_compile("tests/proofs/no-instance/provider_returning_nothing.rs");
    provider_returning_an_instance::main();
}

#[test]
fn a_providers_data_cannot_be_a_non_value() {
    fails_to_compile("tests/proofs/non-value/provider_data.rs");
    provider_data_value::main();
}

#[test]
fn a_providers_event_cannot_carry_a_non_value() {
    fails_to_compile("tests/proofs/non-value/provider_event_data.rs");
    provider_event_data_value::main();
}

#[test]
#[cfg(feature = "async")]
fn a_synchronous_workflow_cannot_have_an_asynchronous_provider() {
    fails_to_compile("tests/proofs/mode-not-accepted/asynchronous_provider.rs");
    asynchronous_provider_async_workflow::main();
}

#[test]
fn a_script_cannot_describe_a_negative_attempt() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/attempt-negative/negative_attempt.rs");
    attempt_counted_from_one::main()
}

#[test]
fn a_script_cannot_describe_a_fraction_of_an_attempt() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/attempt-fraction/fraction_of_an_attempt.rs");
    attempt_counted_from_one::main()
}

#[test]
fn a_step_must_return_an_outcome() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/unexpected-result/step_returning_no_outcome.rs");
    step_returning_an_outcome::main()
}

#[test]
fn a_rescue_step_must_return_an_outcome() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/unexpected-result/rescue_step_returning_no_outcome.rs");
    succeeding_rescue_step::main()
}

#[test]
#[cfg(feature = "async")]
fn an_asynchronous_step_must_resolve_to_an_outcome() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/unexpected-result/asynchronous_step_resolving_to_no_outcome.rs");
    asynchronous_step_async_executor::main()
}

#[test]
fn a_step_hook_cannot_return_something_that_is_not_a_lifecycle() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/unexpected-result/step_hook_returning_not_a_lifecycle.rs");
    fail_workflow_on_step_failure::main()
}

#[test]
fn a_step_hook_must_say_keep_default() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/unexpected-result/step_hook_returning_an_optional_lifecycle.rs");
    fail_workflow_on_step_failure::main()
}

#[test]
fn an_input_adapter_must_return_what_it_supplies() {
    fails_to_compile("tests/proofs/unexpected-result/input_adapter_returning_no_result.rs");
    input_adapter_value::main();
}

#[test]
fn a_provider_cannot_return_an_optional_instance() {
    fails_to_compile("tests/proofs/unexpected-result/provider_returning_an_optional_instance.rs");
    provider_returning_an_instance::main();
}

#[test]
fn a_step_factory_must_build_a_step() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/unexpected-result/step_factory_building_no_step.rs");
    step_factory_building_a_step::main()
}

#[test]
fn a_step_policy_must_build_a_policy() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/unexpected-result/step_policy_building_no_policy.rs");
    step_policy_building_a_policy::main()
}

#[test]
fn a_workflow_policy_must_build_a_policy() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/unexpected-result/workflow_policy_building_no_policy.rs");
    workflow_policy_building_a_policy::main()
}

#[test]
fn an_id_generator_must_return_a_string() -> Result<(), Box<dyn Error>> {
    fails_to_compile("tests/proofs/non-string-journey-id/id_generator_returning_a_number.rs");
    id_generator_returning_a_string::main()
}
