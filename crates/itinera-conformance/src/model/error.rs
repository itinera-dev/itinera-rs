//! The errors of the scenario model: a sentence that does not fit the scenario, or a value it
//! cannot read.

use std::num::NonZeroU32;

use super::{Hook, Lifecycle};

/// A sentence that does not fit the scenario declared so far, or a value it cannot read.
#[derive(Debug, PartialEq, thiserror::Error)]
pub(crate) enum ModelError {
    #[error("no workflow is declared yet")]
    NoWorkflow,
    #[error("a workflow is already declared")]
    WorkflowDeclaredTwice,
    #[error("the workflow has no step \"{0}\"")]
    UnknownStep(String),
    #[error("no policy \"{0}\" is defined")]
    UnknownPolicy(String),
    #[error("no input adapter \"{0}\" is declared")]
    UnknownAdapter(String),
    #[error("the workflow provides no role \"{0}\"")]
    UnknownRole(String),
    #[error("\"{0}\" is not a hook")]
    UnknownHook(String),
    #[error("\"{0}\" is not an event of the catalogue")]
    UnknownEvent(String),
    #[error("\"{0}\" cannot be emitted here")]
    UnknownEmit(String),
    #[error("\"{0}\" is not a type of the vocabulary")]
    UnknownType(String),
    #[error("{0} has no type of the vocabulary")]
    Untyped(String),
    #[error("policy \"{0}\" cannot define \"{1}\", a hook of the other kind")]
    HookOfOtherKind(String, Hook),
    #[error("policy \"{0}\" already defines \"{1}\"")]
    HookDefinedTwice(String, Hook),
    #[error("policy \"{0}\" does not define \"{1}\"")]
    HookNotDefined(String, Hook),
    #[error("\"{0}\" is not a step policy")]
    NotStepPolicy(String),
    #[error("\"{0}\" is not a workflow policy")]
    NotWorkflowPolicy(String),
    #[error("the role has no operation \"{0}\"")]
    UnknownOperation(String),
    #[error("{0} is not JSON")]
    NotJson(String),
    #[error("{0} is not a JSON object")]
    NotAnObject(String),
    #[error("the sentence needs a table with a header")]
    MissingTable,
    #[error("a row has no {0}")]
    MissingCell(&'static str),
    #[error("\"{0}\" is not a column of this table")]
    UnknownColumn(String),
    #[error("\"{1}\" is not a valid {0}")]
    Cell(&'static str, String),
    #[error("attempt {0} is out of order; rows count attempts from 1")]
    AttemptOutOfOrder(NonZeroU32),
    #[error("{0} is already stated")]
    StatedTwice(&'static str),
    #[error("a step name cannot be empty")]
    EmptyStepName,
    #[error("policy \"{0}\" defines no hook")]
    NoHooks(String),
    #[error("input adapter \"{0}\" is attached to no step")]
    AdapterWithoutSteps(String),
    #[error("an input adapter \"{0}\" is already declared")]
    AdapterDeclaredTwice(String),
    #[error("the runner lists at most {0} reporters")]
    TooManyReporters(usize),
    #[error("the hook \"{0}\" cannot request {1}")]
    RequestNotAllowed(Hook, String),
    #[error("the hook \"{0}\" cannot return {1}")]
    LifecycleNotAllowed(Hook, Lifecycle),
}
