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

#[derive(Debug, PartialEq)]
pub(crate) enum Answer {
    Value(Value),
    /// The input is read from the data bag.
    Nothing,
    Fails(String),
}
