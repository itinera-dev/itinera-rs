//! The engine both executors share: it runs one journey, emitting its events, and makes its
//! result.

use std::future::Future;
use std::sync::Arc;

use crate::executor::Started;
use crate::instance::WorkflowInstance;
use crate::journey::{JourneyResult, JourneyStatus};
use crate::policy::PolicyName;
use crate::step::{StepDescriptor, StepName};

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
mod metadata;
mod outcome;
mod request;
mod rescue;
mod retry;
mod runner;
mod scan;
#[cfg(any(test, feature = "testing"))]
mod script;
mod services;

#[cfg(feature = "async")]
pub(crate) use asynchronous::Awaited;
pub(crate) use emitter::{Clock, Emitting, Inline};
pub(crate) use guard::Failures;
pub(crate) use hooks::WorkflowPolicies;
pub(crate) use metadata::introduce;
#[cfg(any(test, feature = "testing"))]
pub(crate) use script::run_scripted;

use emitter::{Delivery, Emitter};
use end::{Aborted, End};
use rescue::LeftForRescue;
use runner::{Runner, Steps};
use services::Kept;

/// Runs the journey of the started instance, with its workflow policies, delivering its events
/// through its dispatcher, whose reporters were guarded by its `failures`.
pub(crate) async fn run<I: WorkflowInstance>(
    started: Started<I, impl Delivery>,
    clock: Clock,
) -> JourneyResult {
    run_with(started, clock, Arc::new(Steps)).await
}

async fn run_with<I: WorkflowInstance>(
    started: Started<I, impl Delivery>,
    clock: Clock,
    runner: Arc<dyn Runner>,
) -> JourneyResult {
    let Started {
        instance,
        policies,
        delivery,
        failures,
    } = started;
    let journey = Journey::new(&instance, delivery, failures, clock, runner);
    journey.run(instance, policies).await
}

/// What the scan does after an attempt of a step described by a `StepDescriptor<W, M>` that
/// lives for `'s`, when the journey goes on.
enum Next<'s, W, M> {
    /// The step is done: it succeeded or skipped itself.
    Step,
    /// The step is attempted again.
    Attempt,
    /// A hook of the step returned `FinishWorkflow`: the journey succeeds without the steps left.
    Finish(PolicyName),
    /// The step's rescue step runs with what the failed attempt left, then the step is attempted
    /// again.
    Rescue(&'s StepDescriptor<W, M>, Box<LeftForRescue>),
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
    /// The instances of journey scope the providers made, which attempts borrow while the
    /// journey lends itself to their handles.
    kept: Arc<Kept>,
    /// What builds and runs each attempt's step, or does what the attempt does instead.
    runner: Arc<dyn Runner>,
}

impl<D: Delivery> Journey<D> {
    fn new<I: WorkflowInstance>(
        instance: &I,
        delivery: D,
        failures: Failures,
        clock: Clock,
        runner: Arc<dyn Runner>,
    ) -> Self {
        let descriptor = instance.descriptor();
        Self {
            emitter: Emitter::new(instance.journey_id().clone(), descriptor.name(), clock),
            delivery,
            failures,
            step: None,
            interrupted: None,
            kept: Arc::new(Kept::of(descriptor)),
            runner,
        }
    }

    /// Runs the journey of the instance, and makes its result.
    async fn run<I: WorkflowInstance>(
        mut self,
        mut instance: I,
        policies: WorkflowPolicies<I::Workflow, I::Mode>,
    ) -> JourneyResult {
        let journey_id = instance.journey_id().clone();
        let descriptor = instance.descriptor().clone();
        let status = match self.travel(&mut instance, &descriptor, &policies).await {
            Ok(()) => JourneyStatus::Succeeded {
                data: instance.into_data_bag(),
            },
            Err(End::Failed(failing)) => JourneyStatus::Failed {
                failure: failing.failure,
                data: instance.into_data_bag(),
            },
            Err(End::Aborted(aborted)) => self.abort(*aborted).await,
        };
        JourneyResult { journey_id, status }
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
