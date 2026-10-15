//! What the contributions recorder and the step event reporter of test support kept, apart
//! from events, and what an input adapter called alone returned.

use std::collections::BTreeSet;

use cucumber::gherkin::Step as Sentence;
use cucumber::then;
use itinera::event::Emitted;
use itinera::journey::Contributions;
use itinera::report::GlobalMetadata;
use itinera::value::Value as Storable;
use serde_json::Value;

use super::{as_json, entry, found};
use crate::model::{json, rows};
use crate::sentences::{Unmet, holds};
use crate::value::{ForType, Typed, for_type};
use crate::world::World;
use itinera::workflow::Supplied;

#[then(expr = "the contributions recorder holds:")]
fn the_contributions_recorder_holds(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let expected = rows(sentence)?
        .iter()
        .map(entry)
        .collect::<Result<Vec<_>, _>>()?;
    let contributions = &world.tested.contributions;
    expected
        .iter()
        .try_for_each(|expected| holds_entry(contributions, expected))?;
    let keys: BTreeSet<&str> = contributions.keys().collect();
    let expected_keys: BTreeSet<&str> = expected.iter().map(key_of).collect();
    holds(
        keys == expected_keys,
        format_args!("the contributions recorder to hold the keys {expected_keys:?}"),
        format_args!("{keys:?}"),
    )
}

fn key_of((key, _): &(String, Value)) -> &str {
    key
}

/// Holds when the recorder holds the value under its key, read as the value's type.
fn holds_entry(contributions: &Contributions, (key, value): &(String, Value)) -> Result<(), Unmet> {
    let reading = Contribution { contributions, key };
    let found = for_type(Typed::of(value)?.value_type(), reading)?;
    holds(
        found.as_ref() == Some(value),
        format_args!("the contributions recorder to hold \"{key}\" = {value}"),
        format_args!("{found:?}"),
    )
}

/// Reads a contribution the recorder holds, as the type of the value expected.
struct Contribution<'a> {
    contributions: &'a Contributions,
    key: &'a str,
}

impl ForType for Contribution<'_> {
    type Output = Result<Option<Value>, serde_json::Error>;

    fn of<T: Storable>(self) -> Self::Output {
        found(self.contributions.read::<T>(self.key))
    }
}

#[then(expr = "the step event reporter kept nothing")]
fn the_step_event_reporter_kept_nothing(world: &mut World) -> Result<(), Unmet> {
    let events = &world.tested.step_events;
    kept_nothing(events.emitted(), events.global_metadata())
}

#[then(expr = "the hook event reporter kept nothing")]
fn the_hook_event_reporter_kept_nothing(world: &mut World) -> Result<(), Unmet> {
    let events = &world.tested.hook_events;
    kept_nothing(events.emitted(), events.global_metadata())
}

fn kept_nothing(emitted: &[Emitted], global_metadata: &GlobalMetadata) -> Result<(), Unmet> {
    let entries = global_metadata
        .into_iter()
        .map(as_json)
        .collect::<Result<Vec<_>, _>>()?;
    holds(
        emitted.is_empty() && entries.is_empty(),
        "no event and no global metadata",
        format_args!("the events {emitted:?} and the global metadata {entries:?}"),
    )
}

#[then(expr = "the step event reporter kept the global metadata:")]
fn the_step_event_reporter_kept_the_global_metadata(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let expected = rows(sentence)?
        .iter()
        .map(entry)
        .collect::<Result<Vec<_>, _>>()?;
    let kept = world
        .tested
        .step_events
        .global_metadata()
        .into_iter()
        .map(as_json)
        .collect::<Result<Vec<_>, _>>()?;
    holds(
        kept == expected,
        format_args!("the global metadata {expected:?}, in this order"),
        format_args!("{kept:?}"),
    )
}

#[then(regex = r#"^the input adapter returned (.+)$"#)]
fn the_input_adapter_returned(world: &mut World, value: String) -> Result<(), Unmet> {
    let expected = json(&value)?;
    let returned = match &world.tested.adapter_returned {
        Some(Ok(Supplied::Value(value))) => Some(serde_json::to_value(value)?),
        Some(Ok(Supplied::FromDataBag)) => None,
        Some(Err(error)) => {
            return Err(Unmet::Expected(format!(
                "the input adapter to return {expected}; it failed: {error}"
            )));
        }
        None => return Err(Unmet::Case("no input adapter was called alone".to_owned())),
    };
    holds(
        returned.as_ref() == Some(&expected),
        format_args!("the input adapter to return {expected}"),
        format_args!("{returned:?}"),
    )
}
