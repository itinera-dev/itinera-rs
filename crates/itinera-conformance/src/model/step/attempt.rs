//! How a scripted step's attempts end, as "step attempts:" writes them.

use std::num::NonZeroU32;

use itinera::error::Error;
use serde_json::Value;

use crate::model::ModelError;
use crate::model::table::Row;
use crate::model::value::{entries, json};

/// How one attempt ends, and what it contributes just before.
#[derive(Debug, PartialEq)]
pub(crate) struct Attempt {
    pub(crate) outcome: AttemptOutcome,
    pub(crate) contributes: Vec<(String, Value)>,
}

impl Attempt {
    pub(crate) fn success() -> Self {
        Self {
            outcome: AttemptOutcome::Success,
            contributes: Vec::new(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum AttemptOutcome {
    Success,
    Failure {
        reason: Reason,
        retriable: bool,
    },
    Skipped(Option<Reason>),
    /// An abnormal termination: an error escapes the step, with this message when one is given.
    Error(Option<String>),
}

/// The reason a step reports with a failure or a skip.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Reason {
    pub(crate) code: String,
    pub(crate) message: Option<String>,
    pub(crate) details: Option<Value>,
}

impl Reason {
    /// The reason a step reported, as the scenario writes one.
    pub(crate) fn written(reason: &itinera::step::Reason) -> Result<Self, Error> {
        Ok(Self {
            code: reason.code().to_owned(),
            message: reason.message().map(str::to_owned),
            details: reason.details().map(serde_json::to_value).transpose()?,
        })
    }
}

/// Reads the rows of "step attempts:", numbered from 1 without gaps.
pub(crate) fn attempts(rows: Vec<Row>) -> Result<Vec<Attempt>, ModelError> {
    rows.iter()
        .zip(1..)
        .map(|(row, expected)| attempt(row, expected))
        .collect()
}

/// Reads one attempt's row, which must have the number expected.
fn attempt(row: &Row, expected: u32) -> Result<Attempt, ModelError> {
    let number: NonZeroU32 = row.parse("attempt")?;
    if number.get() != expected {
        return Err(ModelError::AttemptOutOfOrder(number));
    }
    let outcome = match row.required("outcome")? {
        "success" => AttemptOutcome::Success,
        "failure" => AttemptOutcome::Failure {
            reason: reason(row)?,
            retriable: false,
        },
        "retriable failure" => AttemptOutcome::Failure {
            reason: reason(row)?,
            retriable: true,
        },
        "skipped" => AttemptOutcome::Skipped(match row.optional("code") {
            Some(_) => Some(reason(row)?),
            None => None,
        }),
        "error" => AttemptOutcome::Error(row.optional("message").map(str::to_owned)),
        other => return Err(ModelError::Cell("outcome", other.to_owned())),
    };
    Ok(Attempt {
        outcome,
        contributes: row
            .optional("contributes")
            .map(entries)
            .transpose()?
            .unwrap_or_default(),
    })
}

/// Reads the reason of a failure or a skip from the row's code, message and details.
fn reason(row: &Row) -> Result<Reason, ModelError> {
    Ok(Reason {
        code: row.required("code")?.to_owned(),
        message: row.optional("message").map(str::to_owned),
        details: row.optional("details").map(json).transpose()?,
    })
}
