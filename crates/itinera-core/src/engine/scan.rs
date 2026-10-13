//! The scan: the journey's steps run in their order, each attempted until it is done.

use super::end::End;
use super::{Delivery, Journey, Next, WorkflowPolicies};
use crate::event::EventBody;
use crate::instance::WorkflowInstance;
use crate::policy::PolicyName;
use crate::step::{StepAttempt, StepDescriptor};
use crate::workflow::WorkflowDescriptor;

impl<D: Delivery> Journey<D> {
    /// Runs the journey's steps, then calls the workflow hook for how it ended, and reports it.
    ///
    /// It holds the instance only mutably across an await, so that the journey's future is
    /// `Send` whenever the instance is.
    pub(super) async fn travel<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
        policies: &WorkflowPolicies<I::Workflow, I::Mode>,
    ) -> Result<(), End> {
        let initial_keys = instance.data_bag().keys().map(str::to_owned).collect();
        self.emit(EventBody::JourneyStarted { initial_keys })
            .await?;
        let scanned = self.scan(instance, descriptor).await;
        self.step = None;
        match scanned {
            Ok(decided_by) => {
                self.on_workflow_success(instance, descriptor, policies)
                    .await?;
                self.emit(EventBody::JourneySucceeded { decided_by })
                    .await?;
                Ok(())
            }
            Err(End::Failed(failing)) => {
                self.on_workflow_failure(instance, descriptor, policies)
                    .await?;
                self.emit(EventBody::JourneyFailed {
                    step: failing.step,
                    failure: failing.reported.clone(),
                })
                .await?;
                Err(End::Failed(failing))
            }
            Err(aborted) => Err(aborted),
        }
    }

    /// Runs each step in its order until none is left, or one ends the journey. Returns the
    /// policy whose hook finished the journey early, if one did.
    async fn scan<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        descriptor: &WorkflowDescriptor<I::Workflow, I::Mode>,
    ) -> Result<Option<PolicyName>, End> {
        for step in descriptor.steps() {
            self.step = Some(step.name());
            if let Some(policy) = self.run_step(instance, step).await? {
                return Ok(Some(policy));
            }
        }
        Ok(None)
    }

    /// Attempts a step until it is done, or the journey ends, running its rescue step when an
    /// attempt is handed to it. Returns the policy whose hook finished the journey, if one did.
    async fn run_step<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        step: &StepDescriptor<I::Workflow, I::Mode>,
    ) -> Result<Option<PolicyName>, End> {
        let mut attempt = StepAttempt::first(step.name());
        loop {
            match self.attempt(instance, step, &attempt, None).await? {
                Next::Attempt => attempt = attempt.next(),
                Next::Step => return Ok(None),
                Next::Finish(policy) => return Ok(Some(policy)),
                Next::Rescue(rescue, rescuing) => {
                    self.rescue(instance, rescue, &attempt, *rescuing).await?;
                    attempt = attempt.next();
                }
            }
        }
    }
}
