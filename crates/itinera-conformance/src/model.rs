//! The scenario model: what a scenario's Given sentences declare, as plain data. The runner
//! builds the workflow, its instance and the executor from it, through itinera's public API.

mod adapter;
mod policy;
mod report;
mod step;
mod table;
mod value;

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use serde_json::Value;

pub(crate) use adapter::{Adapter, Answer};
pub(crate) use policy::{Hook, HookAction, HookRequest, HookReturn, HookScript, Policy, Role};
pub(crate) use report::{Dispatching, EventKind, Holding, ReporterFailure};
pub(crate) use step::{Attempt, Input, Step, StepAction, attempts};
pub(crate) use table::{Row, rows};
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

/// A step that does nothing yet, under its name.
fn unscripted(step: String) -> (String, Step) {
    (step, Step::default())
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
            scripts: steps.iter().cloned().map(unscripted).collect(),
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

    /// Fails unless the policy is defined, as a step policy.
    pub(crate) fn require_step_policy(&mut self, policy: &str) -> Result<(), ModelError> {
        if self.policy_mut(policy)?.is_step_policy() {
            Ok(())
        } else {
            Err(ModelError::NotStepPolicy(policy.to_owned()))
        }
    }

    /// Fails unless the policy is defined, as a workflow policy.
    pub(crate) fn require_workflow_policy(&mut self, policy: &str) -> Result<(), ModelError> {
        if self.policy_mut(policy)?.is_step_policy() {
            Err(ModelError::NotWorkflowPolicy(policy.to_owned()))
        } else {
            Ok(())
        }
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
            .find(|declared| declared.is_named(adapter))
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
    #[error("\"{1}\" is not a valid {0}")]
    Cell(&'static str, String),
    #[error("attempt {0} is out of order; rows count attempts from 1")]
    AttemptOutOfOrder(NonZeroU32),
    #[error("{0} is already stated")]
    StatedTwice(&'static str),
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
