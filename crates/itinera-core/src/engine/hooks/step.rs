//! Calling the step hooks of the policies attached to a step, after one of its attempts.

use super::{Called, Decided, Returned};
use crate::engine::attempt::Attempting;
use crate::engine::decision::Failed;
use crate::engine::end::End;
use crate::engine::{Delivery, Journey};
use crate::error::Error;
use crate::event::{self, HookSource, JourneyAbort};
use crate::instance::WorkflowInstance;
use crate::journey::{Abort, Contributions, MissingData};
use crate::policy::{
    BuiltStepPolicy, Call, FailWorkflow, OnSuccess, PolicyName, RetryCause,
    StepAbnormalTermination, StepFailure, StepFailureCause, StepHook, StepHookKind, StepRetry,
    StepSuccess,
};
use crate::step::{Reason, Requirement, StepName};
use crate::value::AnyValue;

/// How the engine calls one step hook of kind `H` on a built policy, which returns `R`.
type StepHookCall<W, M, H, R> = Call<dyn BuiltStepPolicy<W, M>, W, H, M, R>;

/// What a step's attempt left for a step hook to request, and which hook requests it.
pub(crate) struct Left<'s> {
    policy: PolicyName,
    hook: StepHook,
    step: StepName,
    /// What the attempt contributed, committed or not.
    contributed: &'s Contributions,
    /// How the attempt failed, if it did.
    failed: Option<&'s Failed>,
}

impl<'s> Left<'s> {
    pub(super) fn contributed(&self, key: &str) -> Option<AnyValue> {
        self.contributed.get(key).cloned()
    }

    /// The attempt's reason, for a hook that requested it as `requirement`.
    pub(super) fn reason(&self, requirement: Requirement) -> Result<Option<Reason>, End> {
        match (requirement, self.failed.and_then(Failed::reason)) {
            (Requirement::Required, None) => Err(missing_data(
                MissingData::Reason {
                    policy: self.policy,
                    hook: self.hook,
                },
                event::MissingData::Reason {
                    policy: self.policy,
                    hook: self.hook,
                    step: self.step,
                },
            )),
            (_, found) => Ok(found.cloned()),
        }
    }

    /// The attempt's error, for a hook that requested it as `requirement`.
    pub(super) fn error(&self, requirement: Requirement) -> Result<Option<&'s Error>, End> {
        match (requirement, self.failed.and_then(Failed::error)) {
            (Requirement::Required, None) => Err(missing_data(
                MissingData::Error {
                    policy: self.policy,
                    hook: self.hook,
                },
                event::MissingData::Error {
                    policy: self.policy,
                    hook: self.hook,
                    step: self.step,
                },
            )),
            (_, found) => Ok(found),
        }
    }
}

impl<D: Delivery> Journey<D> {
    /// `on step success`, after an attempt that succeeded, once its contributions are committed.
    pub(crate) async fn on_step_success<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
    ) -> Result<Option<Decided<OnSuccess>>, End> {
        self.step_hook::<I, StepSuccess, _>(
            instance,
            attempting,
            StepHook::OnStepSuccess,
            <dyn BuiltStepPolicy<I::Workflow, I::Mode>>::on_step_success,
            attempting.attempt.clone(),
            None,
        )
        .await
    }

    /// `on step failure`, after the step was given up for `cause`.
    pub(crate) async fn on_step_failure<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        failed: &Failed,
        cause: StepFailureCause,
    ) -> Result<Option<Decided<FailWorkflow>>, End> {
        self.step_hook::<I, StepFailure, _>(
            instance,
            attempting,
            StepHook::OnStepFailure,
            <dyn BuiltStepPolicy<I::Workflow, I::Mode>>::on_step_failure,
            (attempting.attempt.clone(), cause),
            Some(failed),
        )
        .await
    }

    /// `on step retry`, once the step's own rule decided to attempt it again for `cause`.
    pub(crate) async fn on_step_retry<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        failed: &Failed,
        cause: RetryCause,
    ) -> Result<Option<Decided<FailWorkflow>>, End> {
        self.step_hook::<I, StepRetry, _>(
            instance,
            attempting,
            StepHook::OnStepRetry,
            <dyn BuiltStepPolicy<I::Workflow, I::Mode>>::on_step_retry,
            (attempting.attempt.clone(), cause),
            Some(failed),
        )
        .await
    }

    /// `on step abnormal termination`, after an attempt that ended in an abnormal termination.
    pub(crate) async fn on_step_abnormal_termination<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        failed: &Failed,
    ) -> Result<Option<Decided<FailWorkflow>>, End> {
        self.step_hook::<I, StepAbnormalTermination, _>(
            instance,
            attempting,
            StepHook::OnStepAbnormalTermination,
            <dyn BuiltStepPolicy<I::Workflow, I::Mode>>::on_step_abnormal_termination,
            attempting.attempt.clone(),
            Some(failed),
        )
        .await
    }

    /// Calls a step hook, if a policy attached to the step defines it.
    async fn step_hook<I: WorkflowInstance, H: StepHookKind, L>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        hook: StepHook,
        call: StepHookCall<I::Workflow, I::Mode, H, Option<L>>,
        context: H::Context,
        failed: Option<&Failed>,
    ) -> Result<Option<Decided<L>>, End>
    where
        Option<L>: Returned,
    {
        let Some((entry, policy)) = attempting.defining(hook) else {
            return Ok(None);
        };
        let name = entry.name();
        let called = Called {
            hook: HookSource::Step {
                policy: name,
                hook,
                step: attempting.attempt.clone(),
            },
            needs: entry.needs(hook),
            context,
            left: Some(Left {
                policy: name,
                hook,
                step: attempting.attempt.step,
                contributed: &attempting.contributed,
                failed,
            }),
        };
        match self.call_hook(instance, policy, call, called).await? {
            Some(lifecycle) => Ok(Some(Decided {
                policy: name,
                lifecycle,
            })),
            None => Ok(None),
        }
    }
}

/// The abort for the reason or the error a step hook required from an attempt that has none.
fn missing_data(missing: MissingData, reported: event::MissingData) -> End {
    End::aborted(
        Abort::RequiredDataMissing(missing),
        JourneyAbort::RequiredDataMissing { missing: reported },
    )
}
