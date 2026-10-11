//! Proofs that rules the specification lets a language make impossible to express cannot be
//! written with itinera: each program breaking the rule fails to compile, and its twin, which
//! keeps the rule, compiles.

fn proves(broken: &str, kept: &str) {
    let cases = trybuild::TestCases::new();
    cases.compile_fail(broken);
    cases.pass(kept);
}

#[test]
fn a_step_cannot_contribute_a_non_value() {
    proves(
        "tests/proofs/non-value/contribution.rs",
        "tests/proofs/non-value/contribution_value.rs",
    );
}

#[test]
fn initial_data_cannot_hold_a_non_value() {
    proves(
        "tests/proofs/non-value/initial_data.rs",
        "tests/proofs/non-value/initial_data_value.rs",
    );
}

#[test]
fn an_input_adapter_cannot_supply_a_non_value() {
    proves(
        "tests/proofs/non-value/input_adapter.rs",
        "tests/proofs/non-value/input_adapter_value.rs",
    );
}

#[test]
fn a_reasons_details_cannot_be_a_non_value() {
    proves(
        "tests/proofs/non-value/reason_details.rs",
        "tests/proofs/non-value/reason_details_value.rs",
    );
}

#[test]
fn a_step_event_cannot_carry_a_non_value() {
    proves(
        "tests/proofs/non-value/event_data.rs",
        "tests/proofs/non-value/event_data_value.rs",
    );
}

#[test]
fn a_failure_hook_cannot_finish_the_workflow() {
    proves(
        "tests/proofs/invalid-lifecycle/finish_workflow_on_step_failure.rs",
        "tests/proofs/invalid-lifecycle/fail_workflow_on_step_failure.rs",
    );
}

#[test]
fn a_rescue_step_cannot_skip() {
    proves(
        "tests/proofs/rescue-skip/skipping_rescue_step.rs",
        "tests/proofs/rescue-skip/succeeding_rescue_step.rs",
    );
}

#[test]
fn a_policy_cannot_be_attached_to_a_workflow_that_does_not_provide_its_role() {
    proves(
        "tests/proofs/role-not-provided/role_not_provided.rs",
        "tests/proofs/role-not-provided/role_provided.rs",
    );
}

#[test]
#[cfg(feature = "async")]
fn a_synchronous_executor_cannot_run_an_asynchronous_step() {
    proves(
        "tests/proofs/mode-not-accepted/asynchronous_step.rs",
        "tests/proofs/mode-not-accepted/asynchronous_step_async_executor.rs",
    );
}

#[test]
fn a_steps_handles_cannot_outlive_its_attempt() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/proofs/late-handle/kept_by_the_factory.rs");
    cases.compile_fail("tests/proofs/late-handle/moved_into_a_thread.rs");
    cases.pass("tests/proofs/late-handle/used_by_a_thread_during_the_attempt.rs");
}

#[test]
fn an_adapters_access_to_the_data_bag_cannot_outlive_its_call() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/proofs/late-handle/kept_by_an_input_adapter.rs");
    cases.compile_fail("tests/proofs/late-handle/kept_by_an_input_adapter_taking_a_static_call.rs");
    cases.pass("tests/proofs/late-handle/read_by_an_input_adapter_during_its_call.rs");
}

#[test]
fn an_adapter_cannot_write_through_its_access_to_the_data_bag() {
    proves(
        "tests/proofs/bag-write/write_through_the_access.rs",
        "tests/proofs/bag-write/read_through_the_access.rs",
    );
}

#[test]
#[cfg(feature = "unstable")]
fn a_journey_scoped_provider_cannot_request_the_attempt() {
    proves(
        "tests/proofs/journey-scope-request/journey_scoped_provider_asks_for_the_attempt.rs",
        "tests/proofs/journey-scope-request/attempt_scoped_provider_asks_for_the_attempt.rs",
    );
}

#[test]
#[cfg(feature = "unstable")]
fn a_provider_cannot_return_nothing() {
    proves(
        "tests/proofs/no-instance/provider_returning_nothing.rs",
        "tests/proofs/no-instance/provider_returning_an_instance.rs",
    );
}

#[test]
#[cfg(feature = "unstable")]
fn a_providers_event_cannot_carry_a_non_value() {
    proves(
        "tests/proofs/non-value/provider_event_data.rs",
        "tests/proofs/non-value/provider_event_data_value.rs",
    );
}

#[test]
#[cfg(all(feature = "async", feature = "unstable"))]
fn a_synchronous_workflow_cannot_have_an_asynchronous_provider() {
    proves(
        "tests/proofs/mode-not-accepted/asynchronous_provider.rs",
        "tests/proofs/mode-not-accepted/asynchronous_provider_async_workflow.rs",
    );
}
