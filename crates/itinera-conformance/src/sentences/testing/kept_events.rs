//! The events the step and hook event reporters of test support kept.

use cucumber::gherkin::Step as Sentence;
use cucumber::then;
use itinera::event::Emitted;
use itinera::value::Value as Storable;
use serde_json::Value;

use crate::model::{Level, ModelError, Row, ValueType, json, rows};
use crate::sentences::{Unmet, holds};
use crate::testing::event_level;
use crate::value::{ForType, for_type};
use crate::world::World;

#[then(expr = "the step event reporter kept the events:")]
fn the_step_event_reporter_kept_the_events(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    kept_the_events(world.tested.step_events.emitted(), sentence)
}

#[then(expr = "the hook event reporter kept the events:")]
fn the_hook_event_reporter_kept_the_events(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    kept_the_events(world.tested.hook_events.emitted(), sentence)
}

/// An event a recorder kept, as a table writes it.
#[derive(Debug)]
struct Kept {
    level: Level,
    message: String,
    /// Its data as JSON, unless the table leaves it out; `None` within when it has none.
    data: Option<Option<Value>>,
}

impl Kept {
    fn of(row: &Row) -> Result<Self, ModelError> {
        Ok(Self {
            level: row.parse("level")?,
            message: row.required("message")?.to_owned(),
            data: row.optional("data").map(data).transpose()?,
        })
    }

    /// Whether the event is this one.
    fn is(&self, emitted: &Emitted) -> Result<bool, Unmet> {
        let data = match &self.data {
            Some(data) => carried(emitted)? == *data,
            None => true,
        };
        Ok(event_level(self.level) == emitted.level() && self.message == emitted.message() && data)
    }
}

fn data(cell: &str) -> Result<Option<Value>, ModelError> {
    match cell {
        "none" => Ok(None),
        written => json(written).map(Some),
    }
}

/// The data an event carries, as JSON, of whichever type of the vocabulary it has.
fn carried(emitted: &Emitted) -> Result<Option<Value>, serde_json::Error> {
    let data = Data { emitted };
    ValueType::ALL
        .into_iter()
        .map(|value_type| for_type(value_type, data))
        .find_map(Result::transpose)
        .transpose()
}

/// Reads an event's data, as one type of the vocabulary.
#[derive(Clone, Copy)]
struct Data<'a> {
    emitted: &'a Emitted,
}

impl ForType for Data<'_> {
    type Output = Result<Option<Value>, serde_json::Error>;

    fn of<T: Storable>(self) -> Self::Output {
        self.emitted
            .data::<T>()
            .map(serde_json::to_value)
            .transpose()
    }
}

/// Holds when the recorder kept exactly the events of the table, in order.
fn kept_the_events(emitted: &[Emitted], sentence: &Sentence) -> Result<(), Unmet> {
    let expected = rows(sentence)?
        .iter()
        .map(Kept::of)
        .collect::<Result<Vec<_>, _>>()?;
    let kept = expected
        .iter()
        .zip(emitted)
        .map(is_kept)
        .collect::<Result<Vec<_>, _>>()?;
    holds(
        expected.len() == emitted.len() && !kept.contains(&false),
        format_args!("the events {expected:?}"),
        format_args!("{emitted:?}"),
    )
}

fn is_kept((expected, emitted): (&Kept, &Emitted)) -> Result<bool, Unmet> {
    expected.is(emitted)
}
