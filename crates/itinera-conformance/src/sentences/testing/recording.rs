//! The recording reporter of test support, and the path read from it.

use std::collections::BTreeMap;

use cucumber::gherkin::Step as Sentence;
use cucumber::{given, then};
use itinera::report::RecordingReporter;

use super::{as_json, entry};
use crate::model::{ModelError, Row, rows};
use crate::sentences::{Unmet, holds, numbered};
use crate::world::World;

#[given(expr = "the instance has the recording reporter of test support")]
fn the_instance_has_the_recording_reporter_of_test_support(world: &mut World) {
    world.model.testing.recording_reporter = true;
}

#[then(expr = "the path is:")]
fn the_path_is(world: &mut World, #[step] sentence: &Sentence) -> Result<(), Unmet> {
    let expected = rows(sentence)?
        .iter()
        .map(line)
        .collect::<Result<Vec<_>, _>>()?;
    path_is(world, &expected)
}

#[then(expr = "the path is empty")]
fn the_path_is_empty(world: &mut World) -> Result<(), Unmet> {
    path_is(world, &[])
}

fn line(row: &Row) -> Result<String, ModelError> {
    Ok(row.required("line")?.to_owned())
}

/// Holds when the path read from the recording reporter is exactly these lines.
fn path_is(world: &World, expected: &[String]) -> Result<(), Unmet> {
    let path = recording(world)?.path();
    holds(
        path == expected,
        format_args!("the path {expected:?}"),
        format_args!("{path:?}"),
    )
}

#[then(
    expr = "the recording reporter of test support kept the same events as the reporter {string}"
)]
fn the_recording_reporter_kept_the_same_events(
    world: &mut World,
    reporter: String,
) -> Result<(), Unmet> {
    let kept: Vec<_> = recording(world)?.events().iter().map(numbered).collect();
    let received: Vec<_> = world
        .recorders
        .get(&reporter)?
        .events()
        .iter()
        .map(numbered)
        .collect();
    holds(
        !kept.is_empty() && kept == received,
        format_args!("the events \"{reporter}\" received, {received:?}"),
        format_args!("{kept:?}"),
    )
}

#[then(expr = "the recording reporter of test support kept the global metadata:")]
fn the_recording_reporter_kept_the_global_metadata(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let expected = rows(sentence)?
        .iter()
        .map(entry)
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let kept = recording(world)?.global_metadata();
    let kept = (&kept)
        .into_iter()
        .map(as_json)
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    holds(
        kept == expected,
        format_args!("the global metadata {expected:?}"),
        format_args!("{kept:?}"),
    )
}

/// The recording reporter of test support the instance was given.
fn recording(world: &World) -> Result<&RecordingReporter, Unmet> {
    world.tested.recording_reporter.as_ref().ok_or_else(|| {
        Unmet::Case("the instance has no recording reporter of test support".to_owned())
    })
}
