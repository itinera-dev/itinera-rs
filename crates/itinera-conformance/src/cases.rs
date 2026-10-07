//! Reading the conformance cases.

use std::path::Path;

use cucumber::gherkin::{Feature, Scenario};
use cucumber::parser::{self, Basic, Parser};
use futures::stream::{self, StreamExt as _};

use crate::executor::Executor;

/// Reads the feature files as cucumber does, then copies every scenario once for each executor
/// that runs it, tagged with that executor, so each executor's run is a scenario of its own.
///
/// The copies are the rows of an Examples table the runner adds. Like the rows of a real one,
/// each has a line of its own, so that the report keeps them apart while they keep the
/// scenario's name: the first copy has the scenario's line, and each next one is moved by the
/// span of lines the scenarios sharing that name cover, such as the rows of a Scenario Outline,
/// past all of them.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Cases;

type Features = stream::Iter<std::vec::IntoIter<parser::Result<Feature>>>;

impl<I: AsRef<Path>> Parser<I> for Cases {
    type Cli = <Basic as Parser<I>>::Cli;
    type Output = stream::Map<Features, fn(parser::Result<Feature>) -> parser::Result<Feature>>;

    fn parse(self, input: I, cli: Self::Cli) -> Self::Output {
        Basic::new()
            .parse(input, cli)
            .map(copied_per_executor as fn(_) -> _)
    }
}

fn copied_per_executor(feature: parser::Result<Feature>) -> parser::Result<Feature> {
    feature.map(per_executor)
}

fn per_executor(mut feature: Feature) -> Feature {
    let inherited = feature.tags.clone();
    feature.scenarios = copies(feature.scenarios, &inherited);
    for rule in &mut feature.rules {
        let inherited: Vec<_> = inherited.iter().chain(&rule.tags).cloned().collect();
        rule.scenarios = copies(std::mem::take(&mut rule.scenarios), &inherited);
    }
    feature
}

fn copies(scenarios: Vec<Scenario>, inherited: &[String]) -> Vec<Scenario> {
    let spans: Vec<usize> = scenarios
        .iter()
        .map(|scenario| span(&scenarios, &scenario.name))
        .collect();
    scenarios
        .into_iter()
        .zip(spans)
        .flat_map(|(scenario, span)| copied(&scenario, span, inherited))
        .collect()
}

/// The number of lines the scenarios with this name cover, from the first to the last.
fn span(scenarios: &[Scenario], name: &str) -> usize {
    let lines = scenarios
        .iter()
        .filter(|scenario| scenario.name == name)
        .map(|scenario| scenario.position.line);
    let first = lines.clone().min().unwrap_or_default();
    let last = lines.max().unwrap_or_default();
    last - first + 1
}

/// A copy of the scenario for each executor that runs it, each moved by `span` lines more.
fn copied(scenario: &Scenario, span: usize, inherited: &[String]) -> Vec<Scenario> {
    Executor::running(inherited.iter().chain(&scenario.tags))
        .iter()
        .zip(0..)
        .map(|(executor, row)| copy(scenario, *executor, row * span))
        .collect()
}

fn copy(scenario: &Scenario, executor: Executor, offset: usize) -> Scenario {
    let mut copy = scenario.clone();
    copy.tags.push(executor.tag().to_owned());
    copy.position.line += offset;
    copy
}
