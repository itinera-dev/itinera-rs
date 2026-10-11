//! Outcomes: what each step was built with and given, and what each hook and service provider
//! received, as the witness of the scenario recorded them.

mod hooks;
mod services;
mod steps;

use std::num::NonZeroU32;

use serde_json::Value;

use crate::model::{Hook, ModelError, Reason};
use crate::witness::{Call, Received};
use crate::world::World;

/// The calls of the policy's hook, in the order they happened.
pub(super) fn calls_of(world: &World, policy: &str, hook: &str) -> Result<Vec<Call>, ModelError> {
    let hook = Hook::named(hook)?;
    Ok(world
        .witnessed()?
        .calls()
        .into_iter()
        .filter(|call| is_of_hook(call, policy, hook))
        .collect())
}

fn is_of_hook(call: &Call, policy: &str, hook: Hook) -> bool {
    call.policy == policy && call.hook == hook
}

/// What a sentence says a hook or a service provider received, on at least one of its calls.
#[derive(Debug)]
enum Expected {
    /// Data from the step under this key, `None` when absent.
    StepData(String, Option<Value>),
    /// Data from the workflow under this key, `None` when absent.
    WorkflowData(String, Option<Value>),
    StepName(String),
    Attempt(NonZeroU32),
    JourneyId(String),
    Cause(String),
    /// The failure reason, `None` when it was absent.
    Reason(Option<Reason>),
    AnError,
    /// An error with this message.
    Error(String),
}

impl Expected {
    fn is_in(&self, got: &Received) -> bool {
        match self {
            Self::StepData(key, value) => got.step_data.get(key) == Some(value),
            Self::WorkflowData(key, value) => got.workflow_data.get(key) == Some(value),
            Self::StepName(step) => got.step_name.as_ref() == Some(step),
            Self::Attempt(attempt) => got.attempt == Some(*attempt),
            Self::JourneyId(id) => got.journey_id.as_ref() == Some(id),
            Self::Cause(cause) => got.cause.as_ref() == Some(cause),
            Self::Reason(reason) => got.reason.as_ref() == Some(reason),
            Self::AnError => got.error.is_some(),
            Self::Error(message) => got.error.as_ref() == Some(message),
        }
    }
}
