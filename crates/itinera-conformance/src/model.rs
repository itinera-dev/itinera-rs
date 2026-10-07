//! The scenario model: what a scenario's Given sentences declare, as plain data. The runner
//! builds the workflow, its instance and the executor from it, through itinera's public API.

mod adapter;
mod policy;
mod report;
mod step;
mod table;
mod value;

use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroU32;

use serde_json::Value;

pub(crate) use adapter::{Adapter, Answer};
pub(crate) use policy::{Hook, HookAction, HookRequest, HookReturn, HookScript, Policy, Role};
pub(crate) use report::{Dispatching, EventKind, Holding, ReporterFailure};
pub(crate) use step::{Attempt, Input, Step, StepAction, attempts};
pub(crate) use table::rows;
pub(crate) use value::{ValueType, json};

/// Everything a scenario declares before it acts.
#[derive(Debug, Default)]
pub(crate) struct Model {
    workflow: Option<Workflow>,
    /// The initial data of the workflow instance, in the order written.
    pub(crate) initial_data: Vec<(String, Value)>,
    pub(crate) policies: BTreeMap<String, Policy>,
    /// How the scripted reporters fail, whether the workflow lists them or a dispatcher holds
    /// them; a reporter not here never fails.
    pub(crate) reporter_failures: BTreeMap<String, ReporterFailure>,
    pub(crate) dispatching: Dispatching,
}

/// The workflow a scenario declares.
#[derive(Debug)]
pub(crate) struct Workflow {
    pub(crate) name: String,
    /// The step names in declaration order, repeated if the scenario declares a name twice.
    pub(crate) steps: Vec<String>,
    /// What each step does, by name, so a repeated name has one script.
    pub(crate) scripts: BTreeMap<String, Step>,
    /// The step policies attached to each step, in order.
    pub(crate) step_policies: BTreeMap<String, Vec<String>>,
    /// The workflow policies attached, in order.
    pub(crate) policies: Vec<String>,
    /// The input adapters, in the order declared.
    pub(crate) adapters: Vec<Adapter>,
    /// The reporters the workflow lists, in order.
    pub(crate) reporters: Vec<String>,
    pub(crate) id_generator: IdGenerator,
    pub(crate) roles: BTreeMap<String, Role>,
}

/// How the workflow instance gets its journey ID.
#[derive(Debug, Default, PartialEq)]
pub(crate) enum IdGenerator {
    /// The default, a UUID version 4.
    #[default]
    Default,
    Returns(String),
}

/// The level of an event a step or hook emits: `step_info` or `journey_info`, and so on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    /// Reads the level of an event kind made of this prefix and the level, such as `step_info`.
    pub(crate) fn of(prefix: &str, kind: &str) -> Result<Self, ModelError> {
        match kind
            .strip_prefix(prefix)
            .and_then(|kind| kind.strip_prefix('_'))
        {
            Some("info") => Ok(Self::Info),
            Some("warning") => Ok(Self::Warning),
            Some("error") => Ok(Self::Error),
            _ => Err(ModelError::UnknownEmit(kind.to_owned())),
        }
    }
}

impl Model {
    pub(crate) fn declare(&mut self, name: String, steps: Vec<String>) -> Result<(), ModelError> {
        if self.workflow.is_some() {
            return Err(ModelError::WorkflowDeclaredTwice);
        }
        self.workflow = Some(Workflow {
            name,
            scripts: steps
                .iter()
                .map(|step| (step.clone(), Step::default()))
                .collect(),
            steps,
            step_policies: BTreeMap::new(),
            policies: Vec::new(),
            adapters: Vec::new(),
            reporters: Vec::new(),
            id_generator: IdGenerator::Default,
            roles: BTreeMap::new(),
        });
        Ok(())
    }

    pub(crate) fn workflow(&self) -> Result<&Workflow, ModelError> {
        self.workflow.as_ref().ok_or(ModelError::NoWorkflow)
    }

    pub(crate) fn workflow_mut(&mut self) -> Result<&mut Workflow, ModelError> {
        self.workflow.as_mut().ok_or(ModelError::NoWorkflow)
    }

    pub(crate) fn step_mut(&mut self, step: &str) -> Result<&mut Step, ModelError> {
        self.workflow_mut()?
            .scripts
            .get_mut(step)
            .ok_or_else(|| ModelError::UnknownStep(step.to_owned()))
    }

    /// Defines a hook in a policy, creating the policy on its first hook.
    pub(crate) fn define(
        &mut self,
        policy: String,
        hook: &str,
        step_policy: bool,
    ) -> Result<(), ModelError> {
        let hook = Hook::named(hook)?;
        if matches!(hook, Hook::Step(_)) != step_policy {
            return Err(ModelError::HookOfOtherKind(policy, hook));
        }
        match self.policies.get_mut(&policy) {
            Some(existing) => existing.define(&policy, hook),
            None => {
                self.policies.insert(policy, Policy::new(hook));
                Ok(())
            }
        }
    }

    pub(crate) fn policy_mut(&mut self, policy: &str) -> Result<&mut Policy, ModelError> {
        self.policies
            .get_mut(policy)
            .ok_or_else(|| ModelError::UnknownPolicy(policy.to_owned()))
    }

    pub(crate) fn hook_mut(
        &mut self,
        policy: &str,
        hook: &str,
    ) -> Result<&mut HookScript, ModelError> {
        let hook = Hook::named(hook)?;
        self.policy_mut(policy)?
            .hook_mut(hook)
            .ok_or_else(|| ModelError::HookNotDefined(policy.to_owned(), hook))
    }

    pub(crate) fn adapter_mut(&mut self, adapter: &str) -> Result<&mut Adapter, ModelError> {
        self.workflow_mut()?
            .adapters
            .iter_mut()
            .find(|declared| declared.name == adapter)
            .ok_or_else(|| ModelError::UnknownAdapter(adapter.to_owned()))
    }

    pub(crate) fn role_mut(&mut self, role: &str) -> Result<&mut Role, ModelError> {
        self.workflow_mut()?
            .roles
            .get_mut(role)
            .ok_or_else(|| ModelError::UnknownRole(role.to_owned()))
    }
}

/// A sentence that does not fit the scenario declared so far, or a value it cannot read.
#[derive(Debug, PartialEq)]
pub(crate) enum ModelError {
    NoWorkflow,
    WorkflowDeclaredTwice,
    UnknownStep(String),
    UnknownPolicy(String),
    UnknownAdapter(String),
    UnknownRole(String),
    UnknownHook(String),
    UnknownEvent(String),
    UnknownEmit(String),
    UnknownType(String),
    HookOfOtherKind(String, Hook),
    HookDefinedTwice(String, Hook),
    HookNotDefined(String, Hook),
    NotStepPolicy(String),
    NotWorkflowPolicy(String),
    UnknownOperation(String),
    NotJson(String),
    NotAnObject(String),
    MissingTable,
    MissingCell(&'static str),
    Cell(&'static str, String),
    AttemptOutOfOrder(NonZeroU32),
    StatedTwice(&'static str),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoWorkflow => write!(f, "no workflow is declared yet"),
            Self::WorkflowDeclaredTwice => write!(f, "a workflow is already declared"),
            Self::UnknownStep(step) => write!(f, "the workflow has no step \"{step}\""),
            Self::UnknownPolicy(policy) => write!(f, "no policy \"{policy}\" is defined"),
            Self::UnknownAdapter(adapter) => {
                write!(f, "no input adapter \"{adapter}\" is declared")
            }
            Self::UnknownRole(role) => write!(f, "the workflow provides no role \"{role}\""),
            Self::UnknownHook(hook) => write!(f, "\"{hook}\" is not a hook"),
            Self::UnknownEvent(event) => write!(f, "\"{event}\" is not an event of the catalogue"),
            Self::UnknownEmit(kind) => write!(f, "\"{kind}\" cannot be emitted here"),
            Self::UnknownType(name) => write!(f, "\"{name}\" is not a type of the vocabulary"),
            Self::HookOfOtherKind(policy, hook) => {
                write!(
                    f,
                    "policy \"{policy}\" cannot define \"{hook}\", a hook of the other kind"
                )
            }
            Self::HookDefinedTwice(policy, hook) => {
                write!(f, "policy \"{policy}\" already defines \"{hook}\"")
            }
            Self::HookNotDefined(policy, hook) => {
                write!(f, "policy \"{policy}\" does not define \"{hook}\"")
            }
            Self::NotStepPolicy(policy) => write!(f, "\"{policy}\" is not a step policy"),
            Self::NotWorkflowPolicy(policy) => write!(f, "\"{policy}\" is not a workflow policy"),
            Self::UnknownOperation(operation) => {
                write!(f, "the role has no operation \"{operation}\"")
            }
            Self::NotJson(text) => write!(f, "{text} is not JSON"),
            Self::NotAnObject(text) => write!(f, "{text} is not a JSON object"),
            Self::MissingTable => write!(f, "the sentence needs a table with a header"),
            Self::MissingCell(column) => write!(f, "a row has no {column}"),
            Self::Cell(column, cell) => write!(f, "\"{cell}\" is not a valid {column}"),
            Self::AttemptOutOfOrder(attempt) => {
                write!(
                    f,
                    "attempt {attempt} is out of order; rows count attempts from 1"
                )
            }
            Self::StatedTwice(what) => write!(f, "{what} is already stated"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_emitted_kind_names_its_level_after_the_prefix_of_whoever_emits_it() {
        assert_eq!(Level::of("step", "step_warning").unwrap(), Level::Warning);
        assert_eq!(Level::of("journey", "journey_error").unwrap(), Level::Error);
        assert!(Level::of("step", "journey_info").is_err());
        assert!(Level::of("step", "step_debug").is_err());
    }

    #[test]
    fn a_step_is_scripted_once_whatever_times_its_name_is_declared() {
        let mut model = Model::default();
        model
            .declare(
                "orders".to_owned(),
                vec!["charge".to_owned(), "charge".to_owned()],
            )
            .unwrap();
        model.step_mut("charge").unwrap().retries = 2;
        let workflow = model.workflow().unwrap();
        assert_eq!(workflow.steps, ["charge", "charge"]);
        assert_eq!(workflow.scripts.len(), 1);
        assert!(model.step_mut("ship").is_err());
    }

    #[test]
    fn a_policys_kind_is_set_by_the_sentence_that_defines_its_first_hook() {
        let mut model = Model::default();
        model
            .define("audit".to_owned(), "on step success", true)
            .unwrap();
        model
            .define("audit".to_owned(), "on step retry", true)
            .unwrap();
        assert!(
            model
                .define("audit".to_owned(), "on workflow success", false)
                .is_err()
        );
        assert!(
            model
                .define("notify".to_owned(), "on step success", false)
                .is_err()
        );
        assert!(model.hook_mut("audit", "on step retry").is_ok());
        assert_eq!(
            model.hook_mut("audit", "on step failure").unwrap_err(),
            ModelError::HookNotDefined("audit".to_owned(), Hook::named("on step failure").unwrap())
        );
    }
}
