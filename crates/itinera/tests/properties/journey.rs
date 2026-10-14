//! Declaring a generated workflow with the builder, and running its one journey on either
//! executor while recording every event.

use std::sync::Arc;

use itinera::event::Event;
#[cfg(feature = "async")]
use itinera::executor::AsyncLocalExecutor;
use itinera::executor::LocalExecutor;
use itinera::instance::Instance;
use itinera::journey::{JourneyResult, StatusKind};
#[cfg(feature = "async")]
use itinera::mode::Asynchronous;
use itinera::mode::{Mode, Synchronous};
use itinera::policy::{StepPolicyDescriptor, WorkflowPolicyDescriptor};
use itinera::report::RecordingReporter;
use itinera::step::{StepDescriptor, StepName, step_name};
use itinera::workflow::{WorkflowBuilder, WorkflowDescriptor};

use crate::plan::{Attempt, Closing, Plan, StepPlan};
use crate::scripted::{Flaky, Scripted, Verdict};

/// The workflow's own type, which holds nothing.
pub(crate) struct Shop;

/// What every value of the journey's data holds, so that it can be told apart from the keys,
/// reasons and messages that events carry.
pub(crate) const BAG_VALUE: &str = "a value of the data bag";

const STEPS: [StepName; 3] = [step_name!("pick"), step_name!("pack"), step_name!("ship")];

/// Each step of the plan with its name, in order.
pub(crate) fn steps(plan: &Plan) -> impl Iterator<Item = (StepName, &StepPlan)> {
    STEPS.into_iter().zip(&plan.steps)
}

/// One journey: every event it emitted, in order, and how it ended.
pub(crate) struct Journey {
    pub(crate) events: Vec<Event>,
    pub(crate) status: StatusKind,
}

/// The journey of the plan, run by the synchronous executor.
pub(crate) fn run(plan: &Plan) -> Journey {
    let workflow = declared(WorkflowDescriptor::builder("shop"), plan);
    let recorder = RecordingReporter::default();
    let result = LocalExecutor::new()
        .run(instance(&workflow, plan, &recorder))
        .unwrap();
    journey(&recorder, &result)
}

/// The journey of the plan, run by the asynchronous executor.
#[cfg(feature = "async")]
pub(crate) fn run_async(plan: &Plan) -> Journey {
    let workflow = declared(WorkflowDescriptor::async_builder("shop"), plan);
    let recorder = RecordingReporter::default();
    let mut executor = AsyncLocalExecutor::new();
    let result =
        futures::executor::block_on(executor.run(instance(&workflow, plan, &recorder))).unwrap();
    journey(&recorder, &result)
}

/// How a mode declares the generated parts of a workflow.
trait Parts: Mode + Sized {
    fn step(name: StepName, step: &StepPlan) -> StepDescriptor<Shop, Self>;
    fn verdict(attempts: Arc<[Attempt]>) -> StepPolicyDescriptor<Verdict, Shop, Self>;
    fn closing(closing: Closing) -> WorkflowPolicyDescriptor<Closing, Shop, Self>;
}

impl Parts for Synchronous {
    fn step(name: StepName, step: &StepPlan) -> StepDescriptor<Shop> {
        StepDescriptor::new(name, Scripted::new(step))
    }

    fn verdict(attempts: Arc<[Attempt]>) -> StepPolicyDescriptor<Verdict, Shop> {
        StepPolicyDescriptor::new("verdict", move || Verdict::new(&attempts))
            .on_step_success()
            .on_step_failure()
            .on_step_retry()
            .on_step_abnormal_termination()
    }

    fn closing(closing: Closing) -> WorkflowPolicyDescriptor<Closing, Shop> {
        WorkflowPolicyDescriptor::new("closing", move || closing)
            .on_workflow_success()
            .on_workflow_failure()
    }
}

#[cfg(feature = "async")]
impl Parts for Asynchronous {
    fn step(name: StepName, step: &StepPlan) -> StepDescriptor<Shop, Asynchronous> {
        StepDescriptor::new_async(name, Scripted::new(step))
    }

    fn verdict(attempts: Arc<[Attempt]>) -> StepPolicyDescriptor<Verdict, Shop, Asynchronous> {
        StepPolicyDescriptor::new_async("verdict", move || Verdict::new(&attempts))
            .on_step_success()
            .on_step_failure()
            .on_step_retry()
            .on_step_abnormal_termination()
    }

    fn closing(closing: Closing) -> WorkflowPolicyDescriptor<Closing, Shop, Asynchronous> {
        WorkflowPolicyDescriptor::new_async("closing", move || closing)
            .on_workflow_success()
            .on_workflow_failure()
    }
}

fn declared<M: Parts>(
    builder: WorkflowBuilder<Shop, M>,
    plan: &Plan,
) -> WorkflowDescriptor<Shop, M> {
    let builder = steps(plan).fold(builder, with_step);
    let builder = match plan.closing {
        Some(closing) => builder.policy(M::closing(closing)),
        None => builder,
    };
    builder.build().unwrap()
}

fn with_step<M: Parts>(
    builder: WorkflowBuilder<Shop, M>,
    (name, step): (StepName, &StepPlan),
) -> WorkflowBuilder<Shop, M> {
    builder.step(described(name, step))
}

fn described<M: Parts>(name: StepName, step: &StepPlan) -> StepDescriptor<Shop, M> {
    let descriptor = M::step(name, step).retry_budget(step.retry_budget);
    let descriptor = if step.retry_on_abnormal_termination {
        descriptor
    } else {
        descriptor.no_retry_on_abnormal_termination()
    };
    if step.has_policy {
        descriptor.policy(M::verdict(step.attempts.clone().into()))
    } else {
        descriptor
    }
}

/// An instance with initial data, whose reporters are the recorder, which never fails, then the
/// flaky reporter.
fn instance<M: Mode>(
    workflow: &WorkflowDescriptor<Shop, M>,
    plan: &Plan,
    recorder: &RecordingReporter,
) -> Instance<Shop, M> {
    workflow
        .instance(Shop)
        .data("order", BAG_VALUE.to_owned())
        .reporter(recorder.clone())
        .reporter(Flaky::failing_on(plan.reporter_fails_on))
        .create()
        .unwrap()
}

/// The journey the recorder saw, ending with the result's status.
fn journey(recorder: &RecordingReporter, result: &JourneyResult) -> Journey {
    Journey {
        events: recorder.events(),
        status: result.status.kind(),
    }
}
