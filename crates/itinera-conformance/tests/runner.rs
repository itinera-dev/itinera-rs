//! The runner reads the cases, selects scenarios with the tag expression, runs each under every
//! executor that accepts it, and writes the Cucumber JSON report.

#![cfg(test)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

struct Run {
    output: Output,
    report: PathBuf,
}

fn run(name: &str, cases: &str, tags: &str) -> Run {
    let report = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}.json"));
    let _ = std::fs::remove_file(&report);
    let output = Command::new(env!("CARGO_BIN_EXE_itinera-conformance"))
        .env(
            "ITINERA_CONFORMANCE_CASES",
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/cases")
                .join(cases),
        )
        .env("ITINERA_CONFORMANCE_TAGS", tags)
        .env("ITINERA_CONFORMANCE_REPORT", &report)
        .output()
        .unwrap();
    Run { output, report }
}

impl Run {
    fn succeeded(&self) -> bool {
        self.output.status.success()
    }

    fn elements(&self) -> Vec<Value> {
        let report: Value = serde_json::from_slice(&std::fs::read(&self.report).unwrap()).unwrap();
        report
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|feature| feature["elements"].as_array().unwrap().clone())
            .collect()
    }

    /// The scenarios in the report, each with its tags.
    fn scenarios(&self) -> Vec<(String, Vec<String>)> {
        self.elements().iter().map(name_and_tags).collect()
    }
}

fn name_and_tags(element: &Value) -> (String, Vec<String>) {
    let tags = element["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tag| tag["name"].as_str().unwrap().to_owned())
        .collect();
    (element["name"].as_str().unwrap().to_owned(), tags)
}

fn scenario(name: &str, tags: &[&str]) -> (String, Vec<String>) {
    let tags = tags.iter().map(|tag| (*tag).to_owned()).collect();
    (name.to_owned(), tags)
}

#[test]
fn a_run_whose_scenarios_all_pass_succeeds_and_writes_the_report() {
    let run = run("passing", "empty", "@fixture");
    assert!(run.succeeded());
    assert!(run.report.exists());
}

#[test]
fn a_scenario_using_an_undefined_sentence_fails_the_run() {
    let run = run("undefined", "runner", "@selected and not @capability-async");
    assert!(!run.succeeded());
}

#[test]
fn only_the_scenarios_the_tag_expression_selects_run() {
    let run = run("selection", "runner", "@fixture and not @selected");
    let names: Vec<_> = run.scenarios().into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["A scenario left out", "A scenario left out"]);
}

#[test]
fn every_scenario_runs_once_under_each_executor_under_its_own_name() {
    let run = run("executors", "runner", "@selected");
    assert_eq!(
        run.scenarios(),
        [
            scenario("A selected scenario", &["selected", "executor-local"]),
            scenario("A selected scenario", &["selected", "executor-async"]),
            scenario(
                "A selected scenario that needs the async capability",
                &["selected", "capability-async", "executor-async"]
            ),
            scenario(
                "A selected scenario that needs the sync capability",
                &["selected", "capability-sync", "executor-local"]
            ),
        ]
    );
}

#[test]
fn the_rows_of_a_scenario_outline_stay_apart_under_both_executors() {
    let run = run("outline", "outline", "@fixture");
    let mut lines: Vec<_> = run
        .elements()
        .iter()
        .map(|element| element["line"].as_u64().unwrap())
        .collect();
    assert_eq!(lines.len(), 8);
    lines.sort_unstable();
    lines.dedup();
    assert_eq!(lines.len(), 8);
}

#[test]
fn a_feature_that_does_not_parse_fails_the_run() {
    let run = run("malformed", "malformed", "@fixture");
    assert!(!run.succeeded());
}

#[test]
fn a_missing_variable_stops_the_runner_before_anything_runs() {
    let report = Path::new(env!("CARGO_TARGET_TMPDIR")).join("missing.json");
    let _ = std::fs::remove_file(&report);
    let output = Command::new(env!("CARGO_BIN_EXE_itinera-conformance"))
        .env_remove("ITINERA_CONFORMANCE_CASES")
        .env("ITINERA_CONFORMANCE_TAGS", "@fixture")
        .env("ITINERA_CONFORMANCE_REPORT", &report)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ITINERA_CONFORMANCE_CASES is not set")
    );
    assert!(!report.exists());
}
