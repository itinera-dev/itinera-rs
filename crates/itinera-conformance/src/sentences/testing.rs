//! Test support: the recording reporter and the path, fakes in place of service providers,
//! scripted runs and their record, and the recorders that lend a step, a hook or an input
//! adapter its handles outside a journey.

mod alone;
mod fakes;
mod interrupted;
mod kept_events;
mod record;
mod recorders;
mod recording;
mod scripts;

use itinera::journey::Read;
use itinera::value::{AnyValue, Value as Storable};
use serde_json::Value;

use crate::model::{ModelError, Row, json};
use crate::sentences::Unmet;

/// A row's key and its value, written as JSON.
fn entry(row: &Row) -> Result<(String, Value), ModelError> {
    Ok((
        row.required("key")?.to_owned(),
        json(row.required("value")?)?,
    ))
}

/// An entry of global metadata, its value written as JSON.
fn as_json((key, value): &(String, AnyValue)) -> Result<(String, Value), Unmet> {
    Ok((key.clone(), serde_json::to_value(value)?))
}

/// What a read found, as JSON: the value, when one of the type asked for is there.
fn found<T: Storable>(read: Read<T>) -> Result<Option<Value>, serde_json::Error> {
    match read {
        Read::Present(value) => serde_json::to_value(value).map(Some),
        Read::Absent | Read::OtherType => Ok(None),
    }
}
