//! Concluding an attempt: acting on the outcome the step reported, or on the error that escaped
//! it, and committing contributions.

use super::attempt::Attempting;
use super::decision::Failed;
use super::end::{End, failed_by_hook};
use super::hooks::Decided;
use super::{Delivery, Journey, Next};
use crate::error::Error;
use crate::event::{EventBody, GiveUpHook, Source};
use crate::instance::{Committed, WorkflowInstance};
use crate::journey::{Contribution, Contributions};
use crate::policy::{OnSuccess, StepHook};
use crate::step::OutcomeKind;

impl<D: Delivery> Journey<D> {
    /// Acts on the outcome a step reported, and on its contributions.
    pub(super) async fn conclude<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        outcome: OutcomeKind,
    ) -> Result<Next, End> {
        let attempt = &attempting.attempt;
        match outcome {
            OutcomeKind::Success => {
                self.emit(EventBody::StepSucceeded {
                    step: attempt.clone(),
                })
                .await?;
                self.commit(
                    instance,
                    Source::Step(attempt.clone()),
                    &attempting.contributed,
                )
                .await?;
                self.succeeded(instance, attempting).await
            }
            OutcomeKind::Skipped(reason) => {
                self.emit(EventBody::StepSkipped {
                    step: attempt.clone(),
                    reason,
                })
                .await?;
                self.emit(EventBody::ContributionsDiscarded {
                    step: attempt.clone(),
                })
                .await?;
                Ok(Next::Step)
            }
            OutcomeKind::Failure(reason) => {
                self.emit(EventBody::StepFailed {
                    step: attempt.clone(),
                    retriable: false,
                    reason: reason.clone(),
                })
                .await?;
                self.failed(instance, attempting, Failed::Failure(reason))
                    .await
            }
            OutcomeKind::RetriableFailure(reason) => {
                self.emit(EventBody::StepFailed {
                    step: attempt.clone(),
                    retriable: true,
                    reason: reason.clone(),
                })
                .await?;
                self.failed(instance, attempting, Failed::RetriableFailure(reason))
                    .await
            }
        }
    }

    /// Commits contributions to the data bag, in order, as coming from `source`.
    pub(super) async fn commit<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        source: Source,
        contributions: &Contributions,
    ) -> Result<(), End> {
        for Contribution { key, value } in contributions {
            let committed = instance.commit(key.clone(), value.clone());
            self.emit(EventBody::ContributionCommitted {
                key: key.clone(),
                source: source.clone(),
            })
            .await?;
            if committed == Committed::Overwritten {
                self.emit(EventBody::DataOverwritten {
                    key: key.clone(),
                    source: source.clone(),
                })
                .await?;
            }
        }
        Ok(())
    }

    /// Acts on what `on step success` returned: the next step by default.
    async fn succeeded<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
    ) -> Result<Next, End> {
        match self.on_step_success(instance, attempting).await? {
            None => Ok(Next::Step),
            Some(Decided {
                policy,
                lifecycle: OnSuccess::FinishWorkflow,
            }) => Ok(Next::Finish(policy)),
            Some(Decided {
                policy,
                lifecycle: OnSuccess::FailWorkflow(reason),
            }) => Err(failed_by_hook(
                attempting.attempt.step,
                policy,
                StepHook::OnStepSuccess,
                reason,
            )),
        }
    }

    /// Acts on an error that escaped a running step: an abnormal termination, after which
    /// `on step abnormal termination` may give the step up.
    pub(super) async fn terminate<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        attempting: &Attempting<'_, I::Workflow, I::Mode>,
        error: Error,
    ) -> Result<Next, End> {
        self.emit(EventBody::StepAbnormalTermination {
            step: attempting.attempt.clone(),
            message: error.to_string(),
        })
        .await?;
        let failed = Failed::AbnormalTermination(error);
        match self
            .on_step_abnormal_termination(instance, attempting, &failed)
            .await?
        {
            Some(decided) => {
                self.given_up_by(attempting, GiveUpHook::OnStepAbnormalTermination, decided)
                    .await
            }
            None => self.failed(instance, attempting, failed).await,
        }
    }
}

#[cfg(test)]
mod tests;
