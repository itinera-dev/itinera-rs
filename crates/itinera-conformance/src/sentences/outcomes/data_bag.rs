//! Outcomes: the data bag, as the result and the event stream show it.

use std::collections::BTreeMap;

use cucumber::gherkin::Step as Sentence;
use cucumber::then;
use itinera::event::{Event, EventBody};
use itinera::journey::JourneyResult;
use itinera::value::AnyValue;
use serde_json::Value;

use crate::model::{Row, json, rows};
use crate::sentences::{Names, Unmet, expect, holds, refused, result, stream};
use crate::trace::Line;
use crate::world::World;

#[then(expr = "the result's data bag contains:")]
fn the_results_data_bag_contains(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    contains(result(world)?, sentence)
}

#[then(expr = "the second journey's data bag contains:")]
fn the_second_journeys_data_bag_contains(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let second = world
        .journeys
        .get(1)
        .ok_or_else(|| Unmet::Case("no second journey ran".to_owned()))?;
    contains(second.run.as_ref().map_err(refused)?, sentence)
}

/// Holds when the result's data bag has every entry of the sentence's table.
fn contains(result: &JourneyResult, sentence: &Sentence) -> Result<(), Unmet> {
    let found = in_json(result)?;
    rows(sentence)?
        .iter()
        .try_for_each(|row| holds_entry(&found, row))
}

fn holds_entry(found: &BTreeMap<&str, Value>, row: &Row) -> Result<(), Unmet> {
    let key = row.required("key")?;
    let value = json(row.required("value")?)?;
    holds(
        found.get(key) == Some(&value),
        format_args!("\"{key}\" = {value} in the data bag"),
        format_args!("{found:?}"),
    )
}

/// The result's data bag, each value as JSON.
fn in_json(result: &JourneyResult) -> Result<BTreeMap<&str, Value>, Unmet> {
    let data = result.status.data().ok_or_else(|| no_data_bag(result))?;
    data.into_iter()
        .map(entry)
        .collect::<Result<_, _>>()
        .map_err(not_json)
}

fn no_data_bag(result: &JourneyResult) -> Unmet {
    Unmet::Expected(format!("a data bag; the journey {}", result.status.kind()))
}

fn entry<'d>((key, value): (&'d String, &AnyValue)) -> Result<(&'d str, Value), serde_json::Error> {
    Ok((key, serde_json::to_value(value)?))
}

fn not_json(error: serde_json::Error) -> Unmet {
    Unmet::Case(error.to_string())
}

#[then(expr = "the result's data bag has no key {string}")]
fn the_results_data_bag_has_no_key(world: &mut World, key: String) -> Result<(), Unmet> {
    let found = in_json(result(world)?)?;
    holds(
        !found.contains_key(key.as_str()),
        format_args!("no \"{key}\" in the data bag"),
        format_args!("{found:?}"),
    )
}

/// The status of an aborted journey has no data bag to read, so the result offers none.
#[then(expr = "the result carries no data bag")]
fn the_result_carries_no_data_bag(world: &mut World) -> Result<(), Unmet> {
    let status = &result(world)?.status;
    holds(
        status.data().is_none(),
        "no data bag",
        format_args!("the data bag of a journey that {}", status.kind()),
    )
}

#[then(expr = "no contribution of {string} was committed")]
fn no_contribution_was_committed(world: &mut World, key: String) -> Result<(), Unmet> {
    let (events, lines) = stream(world)?;
    expect(
        !lines.iter().any(|line| commits(line, &key)),
        format_args!("no contribution_committed of \"{key}\""),
        &events,
    )
}

fn commits(line: &Line, key: &str) -> bool {
    line.event == "contribution_committed" && line.has_cell("key", key)
}

/// `journey_started` holds the keys only, never their values.
#[then(expr = "the journey_started event lists the initial keys {names} without their values")]
fn the_journey_started_event_lists_the_initial_keys(
    world: &mut World,
    keys: Names,
) -> Result<(), Unmet> {
    let mut expected: Vec<String> = keys.into();
    expected.sort();
    let (events, _) = stream(world)?;
    let found = events.iter().find_map(initial_keys);
    expect(
        found.as_ref() == Some(&expected),
        format_args!("journey_started to list {expected:?}"),
        &events,
    )
}

/// The initial keys of a `journey_started`, in order.
fn initial_keys(event: &Event) -> Option<Vec<String>> {
    match &event.body {
        EventBody::JourneyStarted { initial_keys, .. } => {
            let mut keys = initial_keys.clone();
            keys.sort();
            Some(keys)
        }
        _ => None,
    }
}
