//! The engine both executors share: it runs one journey, emitting its events, and makes its
//! result.

use std::future::Future;

use crate::event::EventBody;
use crate::instance::WorkflowInstance;
use crate::journey::{JourneyResult, JourneyStatus};
use crate::policy::PolicyName;
use crate::step::{StepAttempt, StepDescriptor, StepName};
use crate::workflow::WorkflowDescriptor;

#[cfg(feature = "async")]
mod asynchronous;
mod attempt;
mod decision;
mod emitter;
mod end;
#[cfg(test)]
pub(crate) mod fixtures;
mod guard;
mod hooks;
mod inputs;
mod outcome;
mod request;
mod retry;

#[cfg(feature = "async")]
pub(crate) use asynchronous::Awaited;
pub(crate) use emitter::{Clock, Emitted, Emitting, Inline};
pub(crate) use guard::Failures;
pub(crate) use hooks::WorkflowPolicies;

use emitter::{Delivery, Emitter};
use end::{Aborted, End};

/// Runs one journey of the instance, with the workflow policies built for it, delivering its
/// events through the dispatcher, whose reporters were guarded by `failures`.
pub(crate) async fn run<I: WorkflowInstance>(
    mut instance: I,
    policies: WorkflowPolicies<I::Workflow, I::Mode>,
    delivery: impl Delivery,
    failures: Failures,
    clock: Clock,
) -> JourneyResult {
    let journey_id = instance.journey_id().clone();
    let descriptor = instance.descriptor().clone();
    let emitter = Emitter::new(journey_id.clone(), descriptor.name(), clock);
    let mut journey = Journey {
        emitter,
        delivery,
        failures,
        step: None,
        interrupted: None,
    };
    let status = match journey.travel(&mut instance, &descriptor, &policies).await {
        Ok(()) => JourneyStatus::Succeeded {
            data: instance.into_data_bag(),
        },
        Err(End::Failed(failing)) => JourneyStatus::Failed {
            failure: failing.failure,
            data: instance.into_data_bag(),
        },
        Err(End::Aborted(aborted)) => journey.abort(*aborted).await,
    };
    JourneyResult { journey_id, status }
}

/// What the scan does after an attempt, when the journey goes on.
#[derive(Debug)]
enum Next {
    /// The step is done: it succeeded or skipped itself.
    Step,
    /// The step is attempted again.
    Attempt,
    /// A hook of the step returned `FinishWorkflow`: the journey succeeds without the steps left.
    Finish(PolicyName),
}

/// One journey while it runs.
struct Journey<D> {
    emitter: Emitter,
    delivery: D,
    failures: Failures,
    /// The step being run, if any.
    step: Option<StepName>,
    /// The abort recorded when a reporter failed on an event a step or hook emitted, while it
    /// ran. It stands whatever the step or hook does afterwards.
    interrupted: Option<Box<Aborted>>,
}

impl<D: Delivery> Journey<D> {
    /// Runs the journey's steps, then calls the workflow hook for how it ended, and reports it.
    ///
    /// It holds the instance only mutably across an await, so that the journey's future is
    /// `Send` whenever the instance is.
    async fn travel<I: WorkflowInstance>(
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

    /// Attempts a step until it is done, or the journey ends. Returns the policy whose hook
    /// finished the journey, if one did.
    async fn run_step<I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        step: &StepDescriptor<I::Workflow, I::Mode>,
    ) -> Result<Option<PolicyName>, End> {
        let mut attempt = StepAttempt::first(step.name());
        loop {
            match self.attempt(instance, step, &attempt).await? {
                Next::Attempt => attempt = attempt.next(),
                Next::Step => return Ok(None),
                Next::Finish(policy) => return Ok(Some(policy)),
            }
        }
    }
}

/// Runs a future that never waits to its end, without a runtime.
///
/// A synchronous workflow has nothing to wait for, so its journey is ready the first time it is
/// polled.
pub(crate) fn finish<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut context) {
            return value;
        }
    }
}

#[cfg(test)]
mod tests;
