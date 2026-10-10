//! Development tasks for itinera, run with `cargo xtask <task>`.
//!
//! `cargo xtask check` runs every check that CI requires, in order, and stops at the first that
//! fails.

#![forbid(unsafe_code)]

use std::env;
use std::fmt;
use std::io;
use std::process::{Command, ExitCode};

/// The workspace root, where every check runs, whichever directory `cargo xtask` is run from.
const WORKSPACE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// Every check CI requires, in the order `cargo xtask check` runs them.
static CHECKS: [Check; 7] = [
    Check::cargo(&["fmt", "--all", "--check"]),
    Check::cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ]),
    Check::cargo(&["test", "--workspace", "--all-features"]),
    Check::cargo(&["check", "--workspace", "--no-default-features"]),
    Check::cargo(&[
        "+1.85",
        "check",
        "--workspace",
        "--exclude",
        "itinera-conformance",
    ]),
    Check {
        variable: Some(("RUSTDOCFLAGS", "-D warnings")),
        arguments: &["doc", "--workspace", "--no-deps"],
    },
    Check::cargo(&["deny", "check"]),
];

/// A cargo command that must succeed.
#[derive(Debug)]
struct Check {
    /// An environment variable the command runs with, and its value.
    variable: Option<(&'static str, &'static str)>,
    /// What follows `cargo` on the command line.
    arguments: &'static [&'static str],
}

impl Check {
    const fn cargo(arguments: &'static [&'static str]) -> Self {
        Self {
            variable: None,
            arguments,
        }
    }

    fn run(&'static self) -> Result<(), Failure> {
        eprintln!("Running `{self}`");
        match self.command().status() {
            Ok(status) if status.success() => Ok(()),
            Ok(_) => Err(Failure::Failed(self)),
            Err(error) => Err(Failure::NotStarted(self, error)),
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new("cargo");
        command.current_dir(WORKSPACE).args(self.arguments);
        if let Some((name, value)) = self.variable {
            command.env(name, value);
        }
        command
    }
}

impl fmt::Display for Check {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some((name, value)) = self.variable {
            write!(f, "{name}=\"{value}\" ")?;
        }
        write!(f, "cargo {}", self.arguments.join(" "))
    }
}

/// Why a task did not succeed.
#[derive(Debug, thiserror::Error)]
enum Failure {
    #[error("usage: cargo xtask check")]
    Usage,
    #[error("`{0}` could not start: {1}")]
    NotStarted(&'static Check, io::Error),
    #[error("`{0}` failed")]
    Failed(&'static Check),
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match run(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            eprintln!("{failure}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: &[&str]) -> Result<(), Failure> {
    match arguments {
        ["check"] => CHECKS.iter().try_for_each(Check::run),
        _ => Err(Failure::Usage),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    const AGENTS_MANUAL: &str = include_str!("../../../AGENTS.md");

    #[test]
    fn the_checks_are_those_the_agents_manual_requires() {
        let required: Vec<&str> = AGENTS_MANUAL
            .split("## Running the checks")
            .nth(1)
            .unwrap()
            .split("\n## ")
            .next()
            .unwrap()
            .lines()
            .filter_map(listed_command)
            .collect();
        let checks: Vec<String> = CHECKS.iter().map(Check::to_string).collect();

        assert_eq!(checks, required);
    }

    #[rstest]
    #[case::no_task(&[])]
    #[case::another_task(&["build"])]
    #[case::check_with_more_arguments(&["check", "--all"])]
    fn anything_but_check_alone_is_refused(#[case] arguments: &[&str]) {
        assert!(matches!(run(arguments), Err(Failure::Usage)));
    }

    fn listed_command(line: &str) -> Option<&str> {
        line.strip_prefix("- `")?.strip_suffix('`')
    }
}
