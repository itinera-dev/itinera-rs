//! What a scripted step does.

use std::num::NonZeroU32;

use serde_json::Value;

use super::table::Row;
use super::value::{ValueType, entries, json};
use super::{Level, ModelError};

/// A scripted step: what it requests when it is built, what it does on every attempt, and how
/// each attempt ends.
#[derive(Debug, Default)]
pub(crate) struct Step {
    pub(crate) inputs: Vec<Input>,
    /// What every attempt does before it ends, in the order written.
    pub(crate) actions: Vec<StepAction>,
    /// How each attempt ends; attempts after the last repeat it. `None` until a sentence says.
    pub(crate) attempts: Option<Vec<Attempt>>,
    /// The message its constructor fails with, so that it is never built.
    pub(crate) construction_failure: Option<String>,
    pub(crate) retries: u32,
    pub(crate) abnormal_termination_retriable: bool,
    /// Whether it carries on with its script when an emit call is interrupted.
    pub(crate) ignores_failed_emits: bool,
}

/// An input the step declares in its constructor.
#[derive(Debug, PartialEq)]
pub(crate) struct Input {
    pub(crate) key: String,
    pub(crate) value_type: ValueType,
    pub(crate) optional: bool,
}

/// Something a step does during an attempt, before it ends.
#[derive(Debug, PartialEq)]
pub(crate) enum StepAction {
    Contribute {
        key: String,
        value: Value,
    },
    /// Contributes the first value, then changes that same value in place to the second.
    ContributeThenChange {
        key: String,
        value: Value,
        changed: Value,
    },
    /// Changes, in place, the value it received for this input.
    ChangeInput {
        key: String,
        value: Value,
    },
    Emit {
        level: Level,
        message: String,
        data: Option<Value>,
    },
}

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
#[derive(Debug, PartialEq)]
pub(crate) struct Reason {
    pub(crate) code: String,
    pub(crate) message: Option<String>,
    pub(crate) details: Option<Value>,
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
