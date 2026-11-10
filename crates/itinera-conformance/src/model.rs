//! The scenario model: what a scenario's Given sentences declare, as plain data. The runner
//! builds the workflow, its instance and the executor from it, through itinera's public API.

mod adapter;
mod error;
#[cfg(test)]
mod fixtures;
mod level;
mod policy;
mod report;
mod service;
mod step;
mod table;
mod testing;
mod value;
mod workflow;

use std::collections::BTreeMap;

use serde_json::Value;

pub(crate) use adapter::{Adapter, Answer, Answering, answer_for, is_for_step};
pub(crate) use error::ModelError;
pub(crate) use level::Level;
pub(crate) use policy::{
    Hook, HookAction, HookRequest, HookReturn, HookScript, Hooks, Lifecycle, Policy, Role,
};
pub(crate) use report::{Dispatching, EventKind, Holding, ReporterFailure};
pub(crate) use service::{
    Implementation, Provider, ProviderAction, ProviderFailure, Scope, ServiceType,
};
pub(crate) use step::{
    Attempt, AttemptOutcome, Input, Reason, RescueRequest, Step, StepAction, attempts, rescue_of,
};
pub(crate) use table::{Row, rows};
pub(crate) use testing::{Acts, Chosen, Fake, Faking, Played, ScriptRow, ScriptedEnding, Testing};
pub(crate) use value::{ValueType, json};
pub(crate) use workflow::{IdGenerator, Workflow};

/// Everything a scenario declares before it acts.
#[derive(Debug, Default)]
pub(crate) struct Model {
    workflow: Option<Workflow>,
    /// The initial data of the workflow instance, in the order written.
    pub(crate) initial_data: Vec<(String, Value)>,
    /// The initial global metadata of the workflow instance, in the order written.
    pub(crate) initial_global_metadata: Vec<(String, Value)>,
    pub(crate) policies: BTreeMap<String, Policy>,
    /// The input adapters defined and not bound to the workflow yet, in the order defined.
    defined_adapters: Vec<Adapter>,
    /// The service providers defined and not bound to the workflow yet, in the order defined.
    defined_providers: Vec<Provider>,
    /// How the scripted reporters fail, whether the workflow lists them or a dispatcher holds
    /// them; a reporter not here never fails.
    pub(crate) reporter_failures: BTreeMap<String, ReporterFailure>,
    pub(crate) dispatching: Dispatching,
    /// What the scenario asks of the language's test support.
    pub(crate) testing: Testing,
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
            providers: Vec::new(),
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

    /// Gives the step a rescue step, scripted under its rescue step's name, unless a step of the
    /// workflow already has that name.
    pub(crate) fn rescue(&mut self, step: &str) -> Result<(), ModelError> {
        self.step_mut(step)?.rescued = true;
        self.workflow_mut()?
            .scripts
            .entry(rescue_of(step))
            .or_default()
            .rescues = true;
        Ok(())
    }

    /// The script of a rescue step, which only a rescue step's requests may change.
    pub(crate) fn rescue_step_mut(&mut self, step: &str) -> Result<&mut Step, ModelError> {
        let script = self.step_mut(step)?;
        if script.rescues {
            Ok(script)
        } else {
            Err(ModelError::NotRescueStep(step.to_owned()))
        }
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

    /// Every value the scenario sets the key of global metadata to: in the initial global
    /// metadata, or set by a step, a hook or a service provider.
    pub(crate) fn global_metadata_values_of(&self, key: &str) -> Vec<&Value> {
        let initial = self
            .initial_global_metadata
            .iter()
            .filter_map(|entry| value_under(entry, key));
        let steps = self
            .workflow
            .iter()
            .flat_map(|workflow| workflow.scripts.values())
            .flat_map(|step| &step.actions)
            .filter_map(|action| action.global_metadata_for(key));
        let hooks = self
            .policies
            .values()
            .flat_map(Policy::scripts)
            .flat_map(|script| &script.actions)
            .filter_map(|action| action.global_metadata_for(key));
        let providers = self
            .workflow
            .iter()
            .flat_map(|workflow| &workflow.providers)
            .flat_map(|provider| &provider.actions)
            .filter_map(|action| action.global_metadata_for(key));
        initial.chain(steps).chain(hooks).chain(providers).collect()
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

    /// The input adapter of this name: the first the workflow declares, or else the one defined
    /// and not bound yet, which the first sentence about it defines.
    pub(crate) fn adapter_mut(&mut self, adapter: &str) -> Result<&mut Adapter, ModelError> {
        if self.adapter(adapter).is_none() {
            self.defined_adapters
                .push(Adapter::named(adapter.to_owned(), Vec::new()));
        }
        self.workflow
            .iter_mut()
            .flat_map(|workflow| &mut workflow.adapters)
            .chain(&mut self.defined_adapters)
            .find(|declared| declared.is_named(adapter))
            .ok_or_else(|| ModelError::UnknownAdapter(adapter.to_owned()))
    }

    /// The input adapter of this name, as `adapter_mut` finds it, if there is one yet.
    pub(crate) fn adapter(&self, adapter: &str) -> Option<&Adapter> {
        self.workflow
            .iter()
            .flat_map(|workflow| &workflow.adapters)
            .chain(&self.defined_adapters)
            .find(|declared| declared.is_named(adapter))
    }

    /// Declares the input adapter of this name in the workflow, attached to these steps: the one
    /// defined, or else one that answers nothing, as a second declaration of a name does.
    pub(crate) fn bind_adapter(
        &mut self,
        adapter: &str,
        steps: Vec<String>,
    ) -> Result<(), ModelError> {
        let mut bound = match self
            .defined_adapters
            .iter()
            .position(|defined| defined.is_named(adapter))
        {
            Some(position) => self.defined_adapters.remove(position),
            None => Adapter::named(adapter.to_owned(), Vec::new()),
        };
        bound.steps.extend(steps);
        self.workflow_mut()?.adapters.push(bound);
        Ok(())
    }

    /// Defines a service provider, which a later sentence binds to the workflow.
    pub(crate) fn define_provider(&mut self, provider: Provider) {
        self.defined_providers.push(provider);
    }

    /// Declares the service provider of this name defined first and not bound yet in the
    /// workflow, attached to these steps as well.
    pub(crate) fn bind_provider(
        &mut self,
        provider: &str,
        steps: Vec<String>,
    ) -> Result<(), ModelError> {
        let position = self
            .defined_providers
            .iter()
            .position(|defined| defined.is_named(provider))
            .ok_or_else(|| ModelError::NotDefined(provider.to_owned()))?;
        let mut bound = self.defined_providers.remove(position);
        bound.steps.extend(steps);
        self.workflow_mut()?.providers.push(bound);
        Ok(())
    }

    /// The service provider of this name, declared or defined; when two have it, the one
    /// declared or defined last.
    pub(crate) fn provider_mut(&mut self, provider: &str) -> Result<&mut Provider, ModelError> {
        self.workflow
            .iter_mut()
            .flat_map(|workflow| &mut workflow.providers)
            .chain(&mut self.defined_providers)
            .rev()
            .find(|declared| declared.is_named(provider))
            .ok_or_else(|| ModelError::UnknownProvider(provider.to_owned()))
    }

    /// Fails when an input adapter or a service provider is defined but never bound.
    pub(crate) fn require_bound(&self) -> Result<(), ModelError> {
        if let Some(adapter) = self.defined_adapters.first() {
            return Err(ModelError::NotBound("input adapter", adapter.name.clone()));
        }
        match self.defined_providers.first() {
            Some(provider) => Err(ModelError::NotBound(
                "service provider",
                provider.name.clone(),
            )),
            None => Ok(()),
        }
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
mod tests;
