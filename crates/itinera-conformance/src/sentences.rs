//! The sentences of the catalogue, grouped as the catalogue groups them. A sentence used only
//! by scenarios that Rust makes impossible to express is not defined, since those never run.

mod admission;
mod building;
mod data;
mod events;
mod executor;
mod hooks;
mod input_adapters;
mod journey_ids;
mod outcomes;
mod policies;
mod reporters;

use std::fmt;
use std::num::NonZeroU64;
use std::str::FromStr;

use itinera::event::Event;
use itinera::executor::Refusal;
use itinera::journey::{JourneyId, JourneyResult};

use crate::journey::Journey;
use crate::model::ModelError;
use crate::record::NoStream;
use crate::trace::Line;
use crate::world::World;

/// One or more names, each in double quotes, separated by commas: `"a", "b"`.
#[derive(
    Debug,
    PartialEq,
    cucumber::Parameter,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
    derive_more::IntoIterator,
)]
#[param(name = "names", regex = r#""[^"]*"(?:, "[^"]*")*"#)]
struct Names {
    names: Vec<String>,
}

impl FromStr for Names {
    type Err = ModelError;

    fn from_str(text: &str) -> Result<Self, ModelError> {
        text.split(", ")
            .map(unquoted)
            .collect::<Result<Vec<_>, _>>()
            .map(Self::from)
    }
}

fn unquoted(name: &str) -> Result<String, ModelError> {
    name.strip_prefix('"')
        .and_then(|name| name.strip_suffix('"'))
        .map(str::to_owned)
        .ok_or_else(|| ModelError::Cell("name", name.to_owned()))
}

/// Why a Then sentence does not hold.
#[derive(Debug, thiserror::Error)]
enum Unmet {
    /// The implementation did not do what the sentence says.
    #[error("{0}")]
    Expected(String),
    /// The case cannot be checked as written.
    #[error("the case is in error: {0}")]
    Case(String),
}

impl From<ModelError> for Unmet {
    fn from(error: ModelError) -> Self {
        Self::Case(error.to_string())
    }
}

impl From<NoStream> for Unmet {
    fn from(error: NoStream) -> Self {
        Self::Case(error.to_string())
    }
}

/// Holds when the condition does, and otherwise says what was expected and what happened.
fn expect(condition: bool, expected: impl fmt::Display, events: &[Event]) -> Result<(), Unmet> {
    if condition {
        Ok(())
    } else {
        let kinds: Vec<_> = events.iter().map(Event::kind).collect();
        Err(Unmet::Expected(format!(
            "expected {expected}; the events were {kinds:?}"
        )))
    }
}

/// Holds when the condition does, and otherwise says what was expected and what was found.
fn holds(
    condition: bool,
    expected: impl fmt::Display,
    found: impl fmt::Display,
) -> Result<(), Unmet> {
    if condition {
        Ok(())
    } else {
        Err(Unmet::Expected(format!(
            "expected {expected}; found {found}"
        )))
    }
}

/// The stream the sentences about the whole event stream read, as table lines: the events of
/// the scenario's last journey, or every event when no journey ran.
fn stream(world: &World) -> Result<(Vec<Event>, Vec<Line>), Unmet> {
    let events = world.recorders.stream(&world.model)?.events();
    let events = match world.journeys.last() {
        Some(journey) => of_journey(events, &journey.journey_id),
        None => events,
    };
    let lines = events.iter().map(Line::from).collect();
    Ok((events, lines))
}

/// The events of this journey alone.
fn of_journey(events: Vec<Event>, journey_id: &JourneyId) -> Vec<Event> {
    events
        .into_iter()
        .filter(|event| is_of(event, journey_id))
        .collect()
}

fn is_of(event: &Event, journey_id: &JourneyId) -> bool {
    event.journey_id == *journey_id
}

/// The event's sequence number and its line, so that two streams compare whole.
fn numbered(event: &Event) -> (NonZeroU64, Line) {
    (event.sequence, Line::from(event))
}

/// The journey the scenario ran last, which is its only one unless it ran the workflow twice.
fn journey(world: &World) -> Result<&Journey, Unmet> {
    world
        .journeys
        .last()
        .ok_or_else(|| Unmet::Case("no journey ran".to_owned()))
}

/// What `run` returned for the last journey, when it returned a result.
fn result(world: &World) -> Result<&JourneyResult, Unmet> {
    journey(world)?.run.as_ref().map_err(refused)
}

fn refused(refusal: &Refusal) -> Unmet {
    Unmet::Expected(format!("a journey result; run was refused: {refusal}"))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::two_names(r#""audit", "metrics""#, &["audit", "metrics"])]
    #[case::one_name(r#""audit""#, &["audit"])]
    fn names_are_read_from_their_quotes_in_order(#[case] text: &str, #[case] names: &[&str]) {
        let names: Vec<String> = names.iter().copied().map(str::to_owned).collect();
        assert_eq!(text.parse::<Names>().unwrap(), Names::from(names));
    }
}
