//! What each reporter received of the global metadata.

use cucumber::gherkin::Step as Sentence;
use cucumber::then;
use serde_json::Value;

use crate::model::{ModelError, Row, json, rows};
use crate::record::Entry;
use crate::sentences::{Names, Unmet, holds};
use crate::world::World;

/// An entry a reporter received, as a table writes it: its value as JSON, and the kind of the
/// event it came after, or `none`.
#[derive(Debug, PartialEq)]
struct Received {
    key: String,
    value: Value,
    after: String,
}

impl Received {
    fn of(entry: &Entry) -> Result<Self, Unmet> {
        Ok(Self {
            key: entry.key.clone(),
            value: serde_json::to_value(&entry.value)?,
            after: entry.after.unwrap_or("none").to_owned(),
        })
    }
}

/// A row of the expected entries: a cell left empty is not compared.
#[derive(Debug)]
struct Expected {
    key: String,
    value: Option<Value>,
    after: Option<String>,
}

impl Expected {
    fn of(row: &Row) -> Result<Self, ModelError> {
        Ok(Self {
            key: row.required("key")?.to_owned(),
            value: row.optional("value").map(json).transpose()?,
            after: row.optional("after").map(str::to_owned),
        })
    }

    fn is_met_by(&self, received: &Received) -> bool {
        self.key == received.key
            && agrees(self.value.as_ref(), &received.value)
            && agrees(self.after.as_ref(), &received.after)
    }
}

/// Whether a cell has what was found, if it is stated.
fn agrees<T: PartialEq>(stated: Option<&T>, found: &T) -> bool {
    match stated {
        Some(stated) => stated == found,
        None => true,
    }
}

fn is_met((expected, received): (&Expected, &Received)) -> bool {
    expected.is_met_by(received)
}

/// The entries the named reporter received, as tables write them.
fn received_by(world: &World, reporter: &str) -> Result<Vec<Received>, Unmet> {
    world
        .recorders
        .get(reporter)?
        .entries()
        .iter()
        .map(Received::of)
        .collect()
}

#[then(expr = "the reporter {string} received the global metadata:")]
fn the_reporter_received_the_global_metadata(
    world: &mut World,
    reporter: String,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let expected = rows(sentence)?
        .iter()
        .map(Expected::of)
        .collect::<Result<Vec<_>, _>>()?;
    let received = received_by(world, &reporter)?;
    holds(
        expected.len() == received.len() && expected.iter().zip(&received).all(is_met),
        format_args!("\"{reporter}\" to receive the global metadata {expected:?}"),
        format_args!("{received:?}"),
    )
}

#[then(expr = "the reporters {names} received the same global metadata")]
fn the_reporters_received_the_same_global_metadata(
    world: &mut World,
    reporters: Names,
) -> Result<(), Unmet> {
    let mut received = reporters
        .as_ref()
        .iter()
        .map(|name| named_entries(world, name));
    let Some(first) = received.next().transpose()? else {
        return Ok(());
    };
    received.try_for_each(|other| received_the_same(other, &first))
}

/// The name of a reporter and the entries it received.
type Named<'a> = (&'a str, Vec<Received>);

fn named_entries<'a>(world: &World, name: &'a str) -> Result<Named<'a>, Unmet> {
    Ok((name, received_by(world, name)?))
}

/// Holds when the other reporter's entries could be read and are the same as the first's.
fn received_the_same(other: Result<Named<'_>, Unmet>, first: &Named<'_>) -> Result<(), Unmet> {
    let (name, entries) = other?;
    holds(
        entries == first.1,
        format_args!(
            "\"{name}\" to receive the global metadata \"{}\" received, {:?}",
            first.0, first.1
        ),
        format_args!("{entries:?}"),
    )
}
