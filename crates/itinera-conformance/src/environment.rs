//! The environment variables that tell the runner what to run and where to report.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

use cucumber::gherkin::tagexpr::TagOperation;

const CASES: &str = "ITINERA_CONFORMANCE_CASES";
const TAGS: &str = "ITINERA_CONFORMANCE_TAGS";
const REPORT: &str = "ITINERA_CONFORMANCE_REPORT";

/// What one run of the runner reads from its environment.
#[derive(Debug)]
pub(crate) struct Environment {
    /// The directory holding the feature files, searched recursively.
    pub(crate) cases: PathBuf,
    /// Selects the scenarios that run, from the tags of each scenario, its rule and its feature.
    pub(crate) tags: TagOperation,
    /// The file the Cucumber JSON report is written to, replacing what it held.
    pub(crate) report: PathBuf,
}

impl Environment {
    pub(crate) fn from_process() -> Result<Self, EnvironmentError> {
        Self::read(|name| std::env::var_os(name))
    }

    fn read(var: impl Fn(&str) -> Option<OsString>) -> Result<Self, EnvironmentError> {
        let tags = required(&var, TAGS)?
            .into_string()
            .map_err(|_| EnvironmentError::TagsNotUnicode)?;
        Ok(Self {
            cases: required(&var, CASES)?.into(),
            tags: tags
                .parse()
                .map_err(|_| EnvironmentError::InvalidTags(tags))?,
            report: required(&var, REPORT)?.into(),
        })
    }
}

/// The value of a variable that must be set.
fn required(
    var: impl Fn(&str) -> Option<OsString>,
    name: &'static str,
) -> Result<OsString, EnvironmentError> {
    var(name)
        .filter(is_set)
        .ok_or(EnvironmentError::Missing(name))
}

/// Whether a variable holds a value: an empty one counts as missing.
fn is_set(value: &OsString) -> bool {
    !value.is_empty()
}

/// Why the environment does not say what to run.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EnvironmentError {
    Missing(&'static str),
    TagsNotUnicode,
    InvalidTags(String),
}

impl fmt::Display for EnvironmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "{name} is not set"),
            Self::TagsNotUnicode => write!(f, "{TAGS} is not valid Unicode"),
            Self::InvalidTags(tags) => write!(f, "{TAGS} is not a tag expression: {tags}"),
        }
    }
}

impl std::error::Error for EnvironmentError {}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cucumber::tag::Ext as _;

    use super::*;

    fn read(vars: &[(&str, &str)]) -> Result<Environment, EnvironmentError> {
        let vars: HashMap<_, _> = vars.iter().copied().collect();
        Environment::read(|name| lookup(&vars, name))
    }

    fn lookup(vars: &HashMap<&str, &str>, name: &str) -> Option<OsString> {
        vars.get(name).map(OsString::from)
    }

    fn tags(expression: &str) -> TagOperation {
        read(&[
            (CASES, "cases"),
            (TAGS, expression),
            (REPORT, "report.json"),
        ])
        .unwrap()
        .tags
    }

    #[test]
    fn the_three_variables_say_what_to_run_and_where_to_report() {
        let environment = read(&[
            (CASES, "cases"),
            (TAGS, "@tier-1"),
            (REPORT, "out/cucumber.json"),
        ])
        .unwrap();
        assert_eq!(environment.cases, PathBuf::from("cases"));
        assert_eq!(environment.report, PathBuf::from("out/cucumber.json"));
        assert!(environment.tags.eval(["tier-1"]));
    }

    #[test]
    fn each_variable_is_required() {
        assert_eq!(
            read(&[(TAGS, "@a"), (REPORT, "r")]).unwrap_err(),
            EnvironmentError::Missing(CASES)
        );
        assert_eq!(
            read(&[(CASES, "c"), (REPORT, "r")]).unwrap_err(),
            EnvironmentError::Missing(TAGS)
        );
        assert_eq!(
            read(&[(CASES, "c"), (TAGS, "@a")]).unwrap_err(),
            EnvironmentError::Missing(REPORT)
        );
    }

    #[test]
    fn an_empty_variable_counts_as_missing() {
        assert_eq!(
            read(&[(CASES, "c"), (TAGS, ""), (REPORT, "r")]).unwrap_err(),
            EnvironmentError::Missing(TAGS)
        );
    }

    #[test]
    fn a_tag_expression_that_does_not_parse_is_refused() {
        assert_eq!(
            read(&[(CASES, "c"), (TAGS, "(@a and"), (REPORT, "r")]).unwrap_err(),
            EnvironmentError::InvalidTags("(@a and".to_owned())
        );
    }

    #[test]
    fn a_tag_expression_combines_and_or_not_and_parentheses() {
        let selection = tags("(@proposal-0002 or @proposal-0008) and not @non-value");
        assert!(selection.eval(["tier-1", "proposal-0002"]));
        assert!(selection.eval(["proposal-0008"]));
        assert!(!selection.eval(["proposal-0002", "non-value"]));
        assert!(!selection.eval(["proposal-0009"]));
    }
}
