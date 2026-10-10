//! Rules that Rust's types enforce: each program breaking a rule fails to compile, and its twin,
//! which keeps the rule, compiles and runs.

#![cfg(feature = "unstable")]

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
fn an_executor_runs_one_journey_at_a_time() {
    enforced(
        "tests/ui/run_concurrently.rs",
        "tests/ui/run_one_after_the_other.rs",
    );
}
