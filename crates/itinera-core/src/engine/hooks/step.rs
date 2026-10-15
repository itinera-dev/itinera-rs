//! Calling the step hooks of the policies attached to a step, after one of its attempts.

mod left;

pub(crate) use left::Left;

use super::{Called, Decided, Returned};
use crate::engine::attempt::Attempting;
use crate::engine::decision::Failed;
use crate::engine::end::End;
use crate::engine::{Delivery, Journey};
use crate::event::HookSource;
use crate::instance::WorkflowInstance;
use crate::policy::{
    BuiltStepPolicy, Call, Lifecycle, OnFailure, OnSuccess, RetryCause, StepAbnormalTermination,
    StepFailure, StepFailureCause, StepHook, StepHookKind, StepRetry, StepSuccess,
};
use crate::step::Reason;

/// How the engine calls one step hook of kind `H` on a built policy, which returns `R`.
type StepHookCall<W, M, H, R> = Call<dyn BuiltStepPolicy<W, M>, W, H, M, R>;

/// What a step hook returned, as the decision the engine acts on, `None` when it keeps the
/// default.
trait Decides: Returned {
    type Decision;

    fn decision(self) -> Option<Self::Decision>;
}

impl Decides for OnSuccess {
    type Decision = Lifecycle;

    fn decision(self) -> Option<Lifecycle> {
        self.lifecycle()
    }
}

impl Decides for OnFailure {
    type Decision = Reason;

    fn decision(self) -> Option<Reason> {
        self.failing()
    }
}

impl<D: Delivery> Journey<D> {
    /// `on step success`, after an attempt that succeeded, once its contributions are committed.
    pub(crate) async fn on_step_success<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
    ) -> Result<Option<Decided<Lifecycle>>, End> {
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
    ) -> Result<Option<Decided<Reason>>, End> {
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
    ) -> Result<Option<Decided<Reason>>, End> {
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
    ) -> Result<Option<Decided<Reason>>, End> {
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
    async fn step_hook<I: WorkflowInstance, H: StepHookKind, R: Decides>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        hook: StepHook,
        call: StepHookCall<I::Workflow, I::Mode, H, R>,
        context: H::Context,
        failed: Option<&Failed>,
    ) -> Result<Option<Decided<R::Decision>>, End> {
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
        match self
            .call_hook(instance, policy, call, called)
            .await?
            .decision()
        {
            Some(decision) => Ok(Some(Decided {
                policy: name,
                decision,
            })),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::engine::fixtures::{
        DECLINE, Seen, charge_declined_with_a_contribution, seen, travel_workflow,
    };
    use crate::error::Error;
    use crate::policy::{
        HookNeeds, OnFailure, OnStepFailure, Requested, StepFailure, StepPolicyDescriptor,
    };
    use crate::step::Input;
    use crate::workflow::fixtures::{Orders, orders};

    /// `on step failure`, which records the data from the step it was attached to read, which its
    /// own type does not declare.
    struct Echo {
        key: &'static str,
        seen: Seen,
    }

    impl OnStepFailure<Orders> for Echo {
        fn on_step_failure(
            &self,
            mut got: Requested<'_, Orders, StepFailure>,
        ) -> Result<OnFailure, Error> {
            let value = got.from_step(&Input::<String>::new(self.key))?;
            self.seen.lock().unwrap().push(value);
            Ok(OnFailure::KeepDefault)
        }
    }

    #[test]
    fn a_hook_needs_what_its_attachment_declares() {
        let saw = Seen::default();
        let echo = Arc::clone(&saw);
        let policy = StepPolicyDescriptor::new("echo", move || Echo {
            key: "decline",
            seen: Arc::clone(&echo),
        })
        .on_step_failure_needing(HookNeeds::new().from_step(&DECLINE));

        travel_workflow(orders().step(charge_declined_with_a_contribution().policy(policy)));

        assert_eq!(seen(&saw), ["insufficient funds"]);
    }
}
