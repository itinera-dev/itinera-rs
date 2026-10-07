//! Runs the Itinera conformance cases against itinera.

#![forbid(unsafe_code)]

mod cases;
mod environment;
mod executor;
#[expect(dead_code, reason = "the sentences that act on the model read it")]
mod model;
#[expect(
    dead_code,
    reason = "the sentence that runs the workflow makes the recorders"
)]
mod record;
mod sentences;
mod trace;
mod world;

use std::error::Error;
use std::fs::File;
use std::io;
use std::process::ExitCode;

use cucumber::gherkin::tagexpr::TagOperation;
use cucumber::gherkin::{Feature, Rule, Scenario};
use cucumber::tag::Ext as _;
use cucumber::writer::{self, Stats as _};
use cucumber::{World as _, WriterExt as _};

use crate::cases::Cases;
use crate::environment::Environment;
use crate::world::World;

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the selected scenarios and writes the report, telling whether every one passed.
fn run() -> Result<bool, Box<dyn Error>> {
    let Environment {
        cases,
        tags,
        report,
    } = Environment::from_process()?;
    let report = File::create(report)?;
    let selected = move |feature: &Feature, rule: Option<&Rule>, scenario: &Scenario| {
        selects(&tags, feature, rule, scenario)
    };
    let runtime = tokio::runtime::Builder::new_current_thread().build()?;
    let writer = runtime.block_on(
        World::cucumber::<std::path::PathBuf>()
            .with_parser(Cases)
            .with_writer(
                writer::Basic::raw(io::stdout(), writer::Coloring::Never, 0)
                    .summarized()
                    .tee::<World, _>(writer::Json::for_tee(report))
                    .normalized(),
            )
            .fail_on_skipped()
            .filter_run(cases, selected),
    );
    Ok(!writer.execution_has_failed())
}

/// Whether the tag expression selects the scenario, from its own tags and those it inherits.
fn selects(
    tags: &TagOperation,
    feature: &Feature,
    rule: Option<&Rule>,
    scenario: &Scenario,
) -> bool {
    tags.eval(tags_of(feature, rule, scenario))
}

/// A scenario's own tags, after those it inherits from its feature and its rule.
fn tags_of<'a>(
    feature: &'a Feature,
    rule: Option<&'a Rule>,
    scenario: &'a Scenario,
) -> impl Iterator<Item = &'a String> + Clone {
    feature
        .tags
        .iter()
        .chain(rule.into_iter().flat_map(|rule| &rule.tags))
        .chain(&scenario.tags)
}
