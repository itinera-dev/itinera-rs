//! What an event table says of an event's data, and what an event carries.

use itinera::value::AnyValue;
use serde_json::Value;

/// The data of a line: what a table's `data` cell says, or what an event carries.
#[derive(Debug, Default, PartialEq)]
pub(super) enum Data {
    /// The table has no `data` cell, so any data matches.
    #[default]
    Unstated,
    /// No data: the event carries none, or the table says `none`.
    Absent,
    /// This data, as JSON.
    Is(Value),
}

impl Data {
    /// What an event carries: its data as JSON, if it has any.
    pub(super) fn carried(data: Option<&AnyValue>) -> Self {
        data.and_then(as_json).map_or(Self::Absent, Self::Is)
    }

    /// Whether an event's data matches what a table says of it.
    pub(super) fn admits(&self, event: &Self) -> bool {
        *self == Self::Unstated || self == event
    }

    /// The data itself, when there is some.
    pub(super) fn value(&self) -> Option<&Value> {
        match self {
            Self::Is(value) => Some(value),
            Self::Unstated | Self::Absent => None,
        }
    }
}

/// Event data as JSON, to compare with what a table writes.
fn as_json(data: &AnyValue) -> Option<Value> {
    serde_json::to_value(data).ok()
}
