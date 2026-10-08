//! What scripted input adapters do.

use serde_json::Value;

use super::value::ValueType;

/// An input adapter the workflow declares, and the steps it is attached to.
#[derive(Debug)]
pub(crate) struct Adapter {
    pub(crate) name: String,
    pub(crate) steps: Vec<String>,
    /// Its answer for each key a sentence mentions; it returns nothing for any other key.
    pub(crate) answers: Vec<(String, Answer)>,
    /// Data from the workflow it requests, all required.
    pub(crate) requests: Vec<(String, ValueType)>,
}

impl Adapter {
    pub(crate) fn is_named(&self, name: &str) -> bool {
        self.name == name
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Answer {
    Value(Value),
    /// The input is read from the data bag.
    Nothing,
    Fails(String),
}

/// The value an adapter supplies for this key, when its answer is for it and is a value.
pub(crate) fn answer_for<'a>((name, answer): &'a (String, Answer), key: &str) -> Option<&'a Value> {
    if name == key { answer.value() } else { None }
}

impl Answer {
    /// The value the adapter supplies, if it supplies one.
    pub(crate) fn value(&self) -> Option<&Value> {
        match self {
            Self::Value(value) => Some(value),
            Self::Nothing | Self::Fails(_) => None,
        }
    }
}
