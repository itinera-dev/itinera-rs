//! The event stream as the cases' event tables describe it.

mod causes;
mod from_event;
mod sources;

use std::collections::BTreeMap;

use serde_json::Value;

use crate::model::{EventKind, ModelError, Row, json};

/// The columns an event table may have besides `event`.
const COLUMNS: [&str; 15] = [
    "step",
    "attempt",
    "key",
    "code",
    "retriable",
    "policy",
    "hook",
    "lifecycle",
    "adapter",
    "source",
    "cause",
    "decided by",
    "message",
    "data",
    "error",
];

/// What an event table can say of one event: its kind, and its other columns as text, except
/// `data`, which is compared as JSON.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Line {
    pub(crate) event: String,
    cells: BTreeMap<&'static str, String>,
    data: Option<Value>,
    /// What else the event carries that no column shows: reasons' messages and details, and
    /// the message of an abnormal termination.
    carried: Vec<Value>,
}

impl Line {
    /// Reads one row of an event table, keeping only its non-empty cells.
    pub(crate) fn expected(row: &Row) -> Result<Self, ModelError> {
        let mut line = Self {
            event: EventKind::named(row.required("event")?)?.name().to_owned(),
            ..Self::default()
        };
        if let Some(column) = row.columns().find(is_unknown_column) {
            return Err(ModelError::UnknownColumn(column.to_owned()));
        }
        for column in COLUMNS {
            match (column, row.optional(column)) {
                ("data", Some(data)) => line.data = Some(json(data)?),
                (_, Some(cell)) => {
                    line.cells.insert(column, cell.to_owned());
                }
                (_, None) => {}
            }
        }
        Ok(line)
    }

    /// Whether an event matches every cell this line states.
    pub(crate) fn matches(&self, event: &Self) -> bool {
        self.event == event.event
            && self
                .cells
                .iter()
                .all(|(column, cell)| event.has_cell(column, cell))
            && self.data.as_ref().is_none_or(|data| event.has_data(data))
    }

    /// Whether the event names no input adapter.
    pub(crate) fn names_no_adapter(&self) -> bool {
        !self.cells.contains_key("adapter")
    }

    /// Whether the event has this text in this column.
    pub(crate) fn has_cell(&self, column: &str, cell: &str) -> bool {
        self.cells.get(column).map(String::as_str) == Some(cell)
    }

    /// Whether the event's data is this JSON.
    fn has_data(&self, data: &Value) -> bool {
        self.data.as_ref() == Some(data)
    }

    fn set(&mut self, column: &'static str, cell: impl ToString) {
        self.cells.insert(column, cell.to_string());
    }

    /// Whether the event carries this message, as an emitted event's message, an error's
    /// message or a reason's message.
    pub(crate) fn carries_message(&self, message: &str) -> bool {
        ["message", "error"]
            .iter()
            .any(|column| self.has_cell(column, message))
            || self.carried.iter().any(|value| is_text(value, message))
    }

    /// Whether the event carries any of these values.
    pub(crate) fn carries_any(&self, values: &[&Value]) -> bool {
        values.iter().any(|value| self.carries_value(value))
    }

    /// Whether the event carries this value: within the values it carries, or written within
    /// the text it carries. The names it gives, such as a step's or a key's, are not data.
    pub(crate) fn carries_value(&self, value: &Value) -> bool {
        let text = match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        let mut values = self.data.iter().chain(&self.carried);
        let mut texts = ["message", "error"]
            .iter()
            .filter_map(|column| self.cells.get(column));
        values.clone().any(|carried| holds(carried, value))
            || !text.is_empty()
                && (texts.any(|cell| cell.contains(&text))
                    || values.any(|carried| holds_text(carried, &text)))
    }
}

/// Whether an event table may not have this column.
fn is_unknown_column(column: &&str) -> bool {
    *column != "event" && !COLUMNS.contains(column)
}

fn holds(carried: &Value, value: &Value) -> bool {
    carried == value
        || match carried {
            Value::Array(items) => items.iter().any(|item| holds(item, value)),
            Value::Object(entries) => entries.values().any(|item| holds(item, value)),
            _ => false,
        }
}

/// Whether the value is this text.
fn is_text(value: &Value, text: &str) -> bool {
    value.as_str() == Some(text)
}

fn holds_text(carried: &Value, text: &str) -> bool {
    match carried {
        Value::String(carried) => carried.contains(text),
        Value::Array(items) => items.iter().any(|item| holds_text(item, text)),
        Value::Object(entries) => {
            entries.keys().any(|key| key.contains(text))
                || entries.values().any(|item| holds_text(item, text))
        }
        _ => false,
    }
}

/// Whether the events include these lines in this order, possibly with others in between.
pub(crate) fn includes_in_order(events: &[Line], expected: &[Line]) -> bool {
    let mut events = events.iter();
    expected.iter().all(|line| follows(&mut events, line))
}

/// Whether a later event matches the line, consuming the events up to it.
fn follows<'a>(events: &mut impl Iterator<Item = &'a Line>, line: &Line) -> bool {
    events.any(|event| line.matches(event))
}

/// Whether the events are exactly these lines.
pub(crate) fn are_exactly(events: &[Line], expected: &[Line]) -> bool {
    events.len() == expected.len()
        && expected
            .iter()
            .zip(events)
            .all(|(line, event)| line.matches(event))
}

#[cfg(test)]
mod tests;
