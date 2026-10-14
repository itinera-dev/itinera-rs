//! What a scripted step does.

mod attempt;

use serde_json::Value;

use super::value::ValueType;
use super::{Level, ServiceType};

pub(crate) use attempt::{Attempt, AttemptOutcome, Reason, attempts};

/// A scripted step: what it requests when it is built, what it does on every attempt, and how
/// each attempt ends.
#[derive(Debug, Default)]
pub(crate) struct Step {
    pub(crate) inputs: Vec<Input>,
    /// Whether it has a rescue step, scripted under its rescue step's name.
    pub(crate) rescued: bool,
    /// Whether it is a rescue step.
    pub(crate) rescues: bool,
    /// What it requests from the step it rescues, in order, when it is a rescue step.
    pub(crate) rescue_requests: Vec<RescueRequest>,
    /// The services it requests, by key, in the order written.
    pub(crate) services: Vec<(String, ServiceType)>,
    /// What every attempt does before it ends, in the order written.
    pub(crate) actions: Vec<StepAction>,
    /// How each attempt ends; attempts after the last repeat it. `None` until a sentence says.
    pub(crate) attempts: Option<Vec<Attempt>>,
    /// The message its constructor fails with, so that it is never built.
    pub(crate) construction_failure: Option<String>,
    pub(crate) retries: u16,
    /// Whether its descriptor turns off retrying an abnormal termination.
    pub(crate) no_retry_on_abnormal_termination: bool,
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

/// What a rescue step requests from the step it rescues.
#[derive(Debug, PartialEq)]
pub(crate) enum RescueRequest {
    /// Data the failed attempt contributed.
    Data(Input),
    Reason,
    Error,
}

/// The name of the rescue step of the step with this name.
pub(crate) fn rescue_of(step: &str) -> String {
    format!("{step}_rescue")
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
    SetGlobalMetadata {
        key: String,
        value: Value,
    },
}

impl StepAction {
    /// The values this action gives the key: what it contributes or writes under it.
    pub(crate) fn values_for(&self, key: &str) -> Vec<&Value> {
        match self {
            Self::Contribute { key: name, value } | Self::ChangeInput { key: name, value }
                if name == key =>
            {
                vec![value]
            }
            Self::ContributeThenChange {
                key: name,
                value,
                changed,
            } if name == key => vec![value, changed],
            _ => Vec::new(),
        }
    }

    /// The value this action sets the key of global metadata to, if it sets it.
    pub(crate) fn global_metadata_for(&self, key: &str) -> Option<&Value> {
        match self {
            Self::SetGlobalMetadata { key: name, value } if name == key => Some(value),
            _ => None,
        }
    }
}
