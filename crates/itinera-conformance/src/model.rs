//! The scenario model: what a scenario's Given sentences declare, as plain data. The runner
//! builds the workflow, its instance and the executor from it, through itinera's public API.

mod adapter;
mod error;
mod level;
mod policy;
mod report;
mod step;
mod table;
mod value;
mod workflow;

use std::collections::BTreeMap;

use serde_json::Value;

pub(crate) use adapter::{Adapter, Answer, answer_for};
pub(crate) use error::ModelError;
pub(crate) use level::Level;
pub(crate) use policy::{
    Hook, HookAction, HookRequest, HookReturn, HookScript, Hooks, Lifecycle, Policy, Role,
};
pub(crate) use report::{Dispatching, EventKind, Holding, ReporterFailure};
pub(crate) use step::{Attempt, AttemptOutcome, Input, Reason, Step, StepAction, attempts};
pub(crate) use table::{Row, rows};
pub(crate) use value::{ValueType, json};
pub(crate) use workflow::{IdGenerator, Workflow};

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
