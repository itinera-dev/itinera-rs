//! What the scenario's steps, hooks and roles received and did while its journeys ran, which the
//! Then sentences read.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use serde_json::Value;

use crate::model::{Hook, Reason};

/// A record that every scripted step, hook and role of a scenario writes to; its clones share it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Witness {
    seen: Arc<Mutex<Seen>>,
}

#[derive(Debug, Default)]
struct Seen {
    builds: Vec<Build>,
    calls: Vec<Call>,
    operations: Vec<Operation>,
}

/// One step built for an attempt, with the inputs it received.
#[derive(Clone, Debug)]
pub(crate) struct Build {
    pub(crate) step: String,
    pub(crate) attempt: NonZeroU32,
    /// Each input it declared, as JSON, or `None` when it was absent.
    pub(crate) inputs: BTreeMap<String, Option<Value>>,
}

/// One call of a hook, with what it received.
#[derive(Clone, Debug)]
pub(crate) struct Call {
    pub(crate) policy: String,
    pub(crate) hook: Hook,
    pub(crate) received: Received,
}

/// What a hook received for each of its requests, as JSON or text; what it did not request is
/// `None`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Received {
    /// Data from the step, by key, `None` when it was absent.
    pub(crate) step_data: BTreeMap<String, Option<Value>>,
    /// Data from the workflow, by key, `None` when it was absent.
    pub(crate) workflow_data: BTreeMap<String, Option<Value>>,
    pub(crate) step_name: Option<String>,
    pub(crate) attempt: Option<NonZeroU32>,
    /// The failure cause or the retry cause, as the specification writes it.
    pub(crate) cause: Option<String>,
    /// The failure reason, present or absent.
    pub(crate) reason: Option<Option<Reason>>,
    /// The error's message.
    pub(crate) error: Option<String>,
    pub(crate) journey_id: Option<String>,
}

/// One call of a role's operation.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Operation {
    pub(crate) role: String,
    pub(crate) operation: String,
}

impl Witness {
    fn seen(&self) -> MutexGuard<'_, Seen> {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn built(&self, build: Build) {
        self.seen().builds.push(build);
    }

    pub(crate) fn called(&self, call: Call) {
        self.seen().calls.push(call);
    }

    pub(crate) fn operated(&self, operation: Operation) {
        self.seen().operations.push(operation);
    }

    /// The steps built, in the order they were built.
    pub(crate) fn builds(&self) -> Vec<Build> {
        self.seen().builds.clone()
    }

    /// The hooks called, in the order they were called.
    pub(crate) fn calls(&self) -> Vec<Call> {
        self.seen().calls.clone()
    }

    /// The operations of roles called, in the order they were called.
    pub(crate) fn operations(&self) -> Vec<Operation> {
        self.seen().operations.clone()
    }
}
