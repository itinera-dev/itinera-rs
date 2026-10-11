//! What follows an attempt that did not succeed: the step is rescued, attempted again or given
//! up, as its own rule decides, unless a hook decides otherwise.

use super::attempt::Attempting;
use super::decision::{Decision, Failed, decide, given_up};
use super::end::{End, failed_by_hook};
use super::hooks::Decided;
use super::{Delivery, Journey, Next};
use crate::event::{DecidingHook, EventBody, GiveUpCause, GiveUpHook};
use crate::instance::WorkflowInstance;
use crate::policy::{FailWorkflow, RetryCause, StepFailureCause, StepHook};
use crate::step::Reason;

impl<D: Delivery> Journey<D> {
    /// Acts on an attempt that did not succeed, as the step's own rule decides.
    pub(super) async fn failed<'s, I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'s, I::Workflow, I::Mode>,
        failed: Failed,
    ) -> Result<Next<'s, I::Workflow, I::Mode>, End> {
        match decide(&failed, attempting.attempt.attempt, attempting.step) {
            Decision::Rescue(rescue, cause) => {
                self.rescuing(attempting, rescue, failed, cause).await
            }
            Decision::Retry(cause) => self.retry(instance, attempting, &failed, cause).await,
            Decision::GiveUp(cause) => self.give_up(instance, attempting, failed, cause).await,
        }
    }

    /// Attempts the step again, unless `on step retry` gives it up.
    async fn retry<'s, I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'s, I::Workflow, I::Mode>,
        failed: &Failed,
        cause: RetryCause,
    ) -> Result<Next<'s, I::Workflow, I::Mode>, End> {
        match self
            .on_step_retry(instance, attempting, failed, cause)
            .await?
        {
            Some(decided) => {
                self.given_up_by(attempting, GiveUpHook::OnStepRetry, decided)
                    .await
            }
            None => {
                self.emit(EventBody::StepRetrying {
                    step: attempting.attempt.clone(),
                    cause,
                })
                .await?;
                Ok(Next::Attempt)
            }
        }
    }

    /// Gives the step up as its own rule decided, then fails the journey as
    /// `on step failure` decides.
    async fn give_up<'s, I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'s, I::Workflow, I::Mode>,
        failed: Failed,
        cause: StepFailureCause,
    ) -> Result<Next<'s, I::Workflow, I::Mode>, End> {
        let step = attempting.attempt.step;
        self.emit(EventBody::StepGivenUp {
            step: attempting.attempt.clone(),
            cause: given_up(cause),
        })
        .await?;
        match self
            .on_step_failure(instance, attempting, &failed, cause)
            .await?
        {
            Some(Decided { policy, lifecycle }) => Err(failed_by_hook(
                step,
                policy,
                StepHook::OnStepFailure,
                lifecycle.into(),
            )),
            None => {
                let (reported, failure) = failed.into_failure(cause);
                Err(End::failed(step, reported, failure))
            }
        }
    }

    /// Gives the step up because `hook` returned `FailWorkflow`, and fails the journey with its
    /// reason. `on step failure` is not called.
    pub(super) async fn given_up_by<'s, W, M>(
        &mut self,
        attempting: &Attempting<'s, W, M>,
        hook: GiveUpHook,
        Decided { policy, lifecycle }: Decided<FailWorkflow>,
    ) -> Result<Next<'s, W, M>, End> {
        let reason: Reason = lifecycle.into();
        self.emit(EventBody::StepGivenUp {
            step: attempting.attempt.clone(),
            cause: GiveUpCause::FailWorkflow {
                decided_by: DecidingHook { policy, hook },
                reason: reason.clone(),
            },
        })
        .await?;
        Err(failed_by_hook(
            attempting.attempt.step,
            policy,
            hook.step_hook(),
            reason,
        ))
    }
}

#[cfg(test)]
mod tests;
