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

pub(crate) use adapter::{Adapter, Answer, answer_for};
pub(crate) use policy::{
    Hook, HookAction, HookRequest, HookReturn, HookScript, Hooks, Policy, Role,
};
pub(crate) use report::{Dispatching, EventKind, Holding, ReporterFailure};
pub(crate) use step::{Attempt, AttemptOutcome, Input, Reason, Step, StepAction, attempts};
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

    /// Whether an event of this kind is one a step or hook emits, rather than an engine event.
    pub(crate) fn is_emitted(kind: &str) -> bool {
        Self::of("step", kind).is_ok() || Self::of("journey", kind).is_ok()
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

    /// Every value the scenario gives this key: in the initial data, contributed or written by a
    /// step or a hook, or supplied by an input adapter.
    pub(crate) fn values_of(&self, key: &str) -> Vec<&Value> {
        let initial = self
            .initial_data
            .iter()
            .filter_map(|entry| value_under(entry, key));
        let steps = self
            .workflow
            .iter()
            .flat_map(|workflow| workflow.scripts.values());
        let actions = steps
            .clone()
            .flat_map(|step| &step.actions)
            .flat_map(|action| action.values_for(key));
        let attempts = steps
            .filter_map(|step| step.attempts.as_ref())
            .flatten()
            .flat_map(|attempt| &attempt.contributes)
            .filter_map(|entry| value_under(entry, key));
        let hooks = self
            .policies
            .values()
            .flat_map(Policy::scripts)
            .flat_map(|script| &script.actions)
            .filter_map(|action| action.value_for(key));
        let adapters = self
            .workflow
            .iter()
            .flat_map(|workflow| &workflow.adapters)
            .flat_map(|adapter| &adapter.answers)
            .filter_map(|answer| answer_for(answer, key));
        initial
            .chain(actions)
            .chain(attempts)
            .chain(hooks)
            .chain(adapters)
            .collect()
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
}

/// A step that does nothing yet, under its name.
fn unscripted(step: String) -> (String, Step) {
    (step, Step::default())
}

/// The value of an entry, when it is under this key.
fn value_under<'a>((name, value): &'a (String, Value), key: &str) -> Option<&'a Value> {
    (name == key).then_some(value)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    #[test]
    fn a_keys_values_are_every_value_the_scenario_gives_it() {
        let mut model = Model::default();
        model
            .declare("orders".to_owned(), vec!["charge".to_owned()])
            .unwrap();
        model.initial_data.push(("card".to_owned(), json!("4111")));
        let charge = model.step_mut("charge").unwrap();
        charge.actions.push(StepAction::ContributeThenChange {
            key: "token".to_owned(),
            value: json!("first"),
            changed: json!("second"),
        });
        charge.attempts = Some(vec![Attempt {
            outcome: step::AttemptOutcome::Success,
            contributes: vec![("token".to_owned(), json!("third"))],
        }]);
        model
            .define("record".to_owned(), "on step success", true)
            .unwrap();
        model
            .hook_mut("record", "on step success")
            .unwrap()
            .actions
            .push(HookAction::Contribute {
                key: "token".to_owned(),
                value: json!("fourth"),
            });
        assert_eq!(model.values_of("card"), [&json!("4111")]);
        assert_eq!(
            model.values_of("token"),
            [
                &json!("first"),
                &json!("second"),
                &json!("third"),
                &json!("fourth")
            ]
        );
        assert!(model.values_of("note").is_empty());
    }

    #[rstest]
    #[case::a_step_level("step_warning", true)]
    #[case::a_journey_level("journey_info", true)]
    #[case::a_step_outcome("step_failed", false)]
    #[case::a_journey_end("journey_aborted", false)]
    fn only_step_and_journey_levels_are_emitted_events(#[case] kind: &str, #[case] emitted: bool) {
        assert_eq!(Level::is_emitted(kind), emitted);
    }

    #[rstest]
    #[case::by_a_step("step", "step_warning", Some(Level::Warning))]
    #[case::by_the_journey("journey", "journey_error", Some(Level::Error))]
    #[case::with_the_prefix_of_another("step", "journey_info", None)]
    #[case::at_a_level_outside_the_catalogue("step", "step_debug", None)]
    fn an_emitted_kind_names_its_level_after_the_prefix_of_whoever_emits_it(
        #[case] emitter: &str,
        #[case] kind: &str,
        #[case] level: Option<Level>,
    ) {
        assert_eq!(Level::of(emitter, kind).ok(), level);
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
