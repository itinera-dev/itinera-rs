//! What a hook requests, what it does, and what it returns.

use serde_json::Value;

use crate::model::{Level, ValueType};

/// What a hook requests, what it does, and what it returns.
#[derive(Debug, Default)]
pub(crate) struct HookScript {
    /// Its parameters, in the order written.
    pub(crate) requests: Vec<HookRequest>,
    /// What it does before returning, in the order written.
    pub(crate) actions: Vec<HookAction>,
    pub(crate) returns: HookReturn,
}

#[derive(Debug, PartialEq)]
pub(crate) enum HookRequest {
    StepData {
        key: String,
        value_type: ValueType,
        optional: bool,
    },
    StepName,
    FailureReason {
        optional: bool,
    },
    FailureCause,
    RetryCause,
    Error,
    DataFromWorkflow {
        key: String,
        value_type: ValueType,
        optional: bool,
    },
    AttemptNumber,
    JourneyId,
    Role(String),
}

#[derive(Debug, PartialEq)]
pub(crate) enum HookAction {
    /// Changes, in place, the step data it received under this key.
    ChangeStepData {
        key: String,
        value: Value,
    },
    Contribute {
        key: String,
        value: Value,
    },
    /// Adds 1 to a counter its policy instance holds, from 0, and contributes the count.
    ContributeCallCount {
        key: String,
    },
    Emit {
        level: Level,
        message: String,
    },
    CallRole {
        role: String,
        operation: String,
    },
}

impl HookAction {
    /// The value this action gives the key: what it contributes or writes under it.
    pub(crate) fn value_for(&self, key: &str) -> Option<&Value> {
        match self {
            Self::ChangeStepData { key: name, value } | Self::Contribute { key: name, value }
                if name == key =>
            {
                Some(value)
            }
            _ => None,
        }
    }
}

/// A lifecycle a hook returns, as an error names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, derive_more::Display)]
pub(crate) enum Lifecycle {
    FinishWorkflow,
    FailWorkflow,
}

/// How a hook ends.
#[derive(Debug, Default, PartialEq)]
pub(crate) enum HookReturn {
    /// No lifecycle.
    #[default]
    Nothing,
    FinishWorkflow,
    FailWorkflow {
        code: String,
    },
    /// The hook fails, with this message when one is given.
    Fails(Option<String>),
}
