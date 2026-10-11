//! The execution modes the scenario's workflow can be declared in.

use std::sync::Arc;

use itinera::mode::{Asynchronous, Mode, Synchronous};
use itinera::policy::{
    Hookless, StepHook, StepPolicyDescriptor, WorkflowHook, WorkflowPolicyDescriptor,
};
use itinera::service::{Service, ServiceProviderDescriptor};
use itinera::step::{StepDescriptor, StepName};
use itinera::workflow::{WorkflowBuilder, WorkflowDescriptor};

use super::ScriptedWorkflow;
use super::policy::{StepPolicy, WorkflowPolicy};
use crate::model::ModelError;
use crate::policy::{Building, Scripts};
use crate::service::{Awaiting, Scope, ScriptedProvider, Supplied, provided_now};
use crate::step::Scripted;

/// An execution mode the scenario's workflow can be declared in.
pub(crate) trait Declares: Mode + Sized {
    fn builder(name: &'static str) -> WorkflowBuilder<ScriptedWorkflow, Self>;

    fn step(name: StepName, factory: Scripted) -> StepDescriptor<ScriptedWorkflow, Self>;

    /// The step, with the rescue step the factory builds.
    fn rescue(
        step: StepDescriptor<ScriptedWorkflow, Self>,
        factory: Scripted,
    ) -> StepDescriptor<ScriptedWorkflow, Self>;

    fn step_policy(building: Building) -> StepPolicy<Self, Hookless>;

    /// The step policy, which defines this hook too, needing what its script says.
    fn step_hook<S>(
        policy: StepPolicy<Self, S>,
        hook: StepHook,
        scripts: &Scripts,
    ) -> Result<StepPolicy<Self>, ModelError>;

    fn workflow_policy(building: Building) -> WorkflowPolicy<Self, Hookless>;

    /// The workflow policy, which defines this hook too, needing what its script says.
    fn workflow_hook<S>(
        policy: WorkflowPolicy<Self, S>,
        hook: WorkflowHook,
        scripts: &Scripts,
    ) -> Result<WorkflowPolicy<Self>, ModelError>;

    /// The scripted provider, of scope `S`, supplying the service `T`.
    fn provider<S: Scope, T: Supplied + ?Sized>(
        provider: ScriptedProvider,
        service: &Service<T>,
    ) -> ServiceProviderDescriptor<ScriptedWorkflow, S, Self>;
}

impl Declares for Synchronous {
    fn builder(name: &'static str) -> WorkflowBuilder<ScriptedWorkflow, Self> {
        WorkflowDescriptor::builder(name)
    }

    fn step(name: StepName, factory: Scripted) -> StepDescriptor<ScriptedWorkflow, Self> {
        StepDescriptor::new(name, factory)
    }

    fn rescue(
        step: StepDescriptor<ScriptedWorkflow, Self>,
        factory: Scripted,
    ) -> StepDescriptor<ScriptedWorkflow, Self> {
        step.rescue(factory)
    }

    fn step_policy(building: Building) -> StepPolicy<Self, Hookless> {
        StepPolicyDescriptor::fallible(building.name(), move || building.build())
    }

    fn step_hook<S>(
        policy: StepPolicy<Self, S>,
        hook: StepHook,
        scripts: &Scripts,
    ) -> Result<StepPolicy<Self>, ModelError> {
        Ok(match hook {
            StepHook::OnStepSuccess => policy.on_step_success_needing(scripts.needs()?),
            StepHook::OnStepFailure => policy.on_step_failure_needing(scripts.needs()?),
            StepHook::OnStepRetry => policy.on_step_retry_needing(scripts.needs()?),
            StepHook::OnStepAbnormalTermination => {
                policy.on_step_abnormal_termination_needing(scripts.needs()?)
            }
            _ => return Err(unknown(hook)),
        })
    }

    fn workflow_policy(building: Building) -> WorkflowPolicy<Self, Hookless> {
        WorkflowPolicyDescriptor::fallible(building.name(), move || building.build())
    }

    fn workflow_hook<S>(
        policy: WorkflowPolicy<Self, S>,
        hook: WorkflowHook,
        scripts: &Scripts,
    ) -> Result<WorkflowPolicy<Self>, ModelError> {
        Ok(match hook {
            WorkflowHook::OnWorkflowSuccess => policy.on_workflow_success_needing(scripts.needs()?),
            WorkflowHook::OnWorkflowFailure => policy.on_workflow_failure_needing(scripts.needs()?),
            _ => return Err(unknown(hook)),
        })
    }

    fn provider<S: Scope, T: Supplied + ?Sized>(
        provider: ScriptedProvider,
        service: &Service<T>,
    ) -> ServiceProviderDescriptor<ScriptedWorkflow, S, Self> {
        S::synchronous(provider.name(), service, move |_, got| {
            provided_now(&provider, got)
        })
    }
}

impl Declares for Asynchronous {
    fn builder(name: &'static str) -> WorkflowBuilder<ScriptedWorkflow, Self> {
        WorkflowDescriptor::async_builder(name)
    }

    fn step(name: StepName, factory: Scripted) -> StepDescriptor<ScriptedWorkflow, Self> {
        StepDescriptor::new_async(name, factory)
    }

    fn rescue(
        step: StepDescriptor<ScriptedWorkflow, Self>,
        factory: Scripted,
    ) -> StepDescriptor<ScriptedWorkflow, Self> {
        step.rescue_async(factory)
    }

    fn step_policy(building: Building) -> StepPolicy<Self, Hookless> {
        StepPolicyDescriptor::fallible_async(building.name(), move || building.build())
    }

    fn step_hook<S>(
        policy: StepPolicy<Self, S>,
        hook: StepHook,
        scripts: &Scripts,
    ) -> Result<StepPolicy<Self>, ModelError> {
        Ok(match hook {
            StepHook::OnStepSuccess => policy.on_step_success_needing(scripts.needs()?),
            StepHook::OnStepFailure => policy.on_step_failure_needing(scripts.needs()?),
            StepHook::OnStepRetry => policy.on_step_retry_needing(scripts.needs()?),
            StepHook::OnStepAbnormalTermination => {
                policy.on_step_abnormal_termination_needing(scripts.needs()?)
            }
            _ => return Err(unknown(hook)),
        })
    }

    fn workflow_policy(building: Building) -> WorkflowPolicy<Self, Hookless> {
        WorkflowPolicyDescriptor::fallible_async(building.name(), move || building.build())
    }

    fn workflow_hook<S>(
        policy: WorkflowPolicy<Self, S>,
        hook: WorkflowHook,
        scripts: &Scripts,
    ) -> Result<WorkflowPolicy<Self>, ModelError> {
        Ok(match hook {
            WorkflowHook::OnWorkflowSuccess => policy.on_workflow_success_needing(scripts.needs()?),
            WorkflowHook::OnWorkflowFailure => policy.on_workflow_failure_needing(scripts.needs()?),
            _ => return Err(unknown(hook)),
        })
    }

    fn provider<S: Scope, T: Supplied + ?Sized>(
        provider: ScriptedProvider,
        service: &Service<T>,
    ) -> ServiceProviderDescriptor<ScriptedWorkflow, S, Self> {
        S::asynchronous(provider.name(), service, Awaiting::from(Arc::new(provider)))
    }
}

/// A hook itinera knows that the runner does not.
fn unknown(hook: impl ToString) -> ModelError {
    ModelError::UnknownHook(hook.to_string())
}
