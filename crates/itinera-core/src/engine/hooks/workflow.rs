//! Calling the workflow hook of the workflow's policies for how a journey ended.

use super::Called;
use crate::engine::end::End;
use crate::engine::{Delivery, Journey};
use crate::event::HookSource;
use crate::instance::WorkflowInstance;
use crate::policy::{
    BuiltWorkflowPolicy, Call, HookKind, WorkflowFailure, WorkflowHook, WorkflowSuccess,
};
use crate::workflow::WorkflowDescriptor;

/// The workflow policies built for one journey, in the order they were attached.
pub(crate) type WorkflowPolicies<W, M> = Vec<Box<dyn BuiltWorkflowPolicy<W, M>>>;

/// How the engine calls one workflow hook of kind `H` on a built policy.
type WorkflowHookCall<W, M, H> = Call<dyn BuiltWorkflowPolicy<W, M>, W, H, M, ()>;

impl<D: Delivery> Journey<D> {
    /// `on workflow success`, once the journey succeeded, before `journey_succeeded`.
    pub(crate) async fn on_workflow_success<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
        policies: &WorkflowPolicies<I::Workflow, I::Mode>,
    ) -> Result<(), End> {
        self.workflow_hook::<I, WorkflowSuccess>(
            instance,
            descriptor,
            policies,
            WorkflowHook::OnWorkflowSuccess,
            <dyn BuiltWorkflowPolicy<I::Workflow, I::Mode>>::on_workflow_success,
        )
        .await
    }

    /// `on workflow failure`, once the journey failed, before `journey_failed`.
    pub(crate) async fn on_workflow_failure<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
        policies: &WorkflowPolicies<I::Workflow, I::Mode>,
    ) -> Result<(), End> {
        self.workflow_hook::<I, WorkflowFailure>(
            instance,
            descriptor,
            policies,
            WorkflowHook::OnWorkflowFailure,
            <dyn BuiltWorkflowPolicy<I::Workflow, I::Mode>>::on_workflow_failure,
        )
        .await
    }

    /// Calls a workflow hook, if a policy attached to the workflow defines it.
    async fn workflow_hook<I: WorkflowInstance, H: HookKind<Context = ()>>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
        policies: &WorkflowPolicies<I::Workflow, I::Mode>,
        hook: WorkflowHook,
        call: WorkflowHookCall<I::Workflow, I::Mode, H>,
    ) -> Result<(), End> {
        let defining = descriptor
            .policies()
            .iter()
            .zip(policies)
            .find(|(entry, _)| entry.defines(hook));
        let Some((entry, policy)) = defining else {
            return Ok(());
        };
        let called = Called {
            hook: HookSource::Workflow {
                policy: entry.name(),
                hook,
            },
            needs: entry.needs(hook),
            context: (),
            left: None,
        };
        self.call_hook(instance, policy.as_ref(), call, called)
            .await
    }
}

#[cfg(test)]
mod tests;
