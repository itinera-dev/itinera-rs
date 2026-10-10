//! Running the scenario's workflow: its journeys, one after another, under the executor of the
//! scenario's copy.

use std::collections::BTreeMap;

use itinera::executor::{AsyncLocalExecutor, LocalExecutor, Refusal};
use itinera::instance::{Instance, InstanceBuilder, InstanceError, WorkflowInstance as _};
use itinera::journey::{JourneyId, JourneyResult};
use itinera::mode::{Asynchronous, Mode, Synchronous};
use itinera::report::{AsyncDispatcherFactory, DispatcherFactory};
use itinera::value::Value as Storable;
use serde_json::Value;

use crate::declaration::{Admission, Declares, ScriptedWorkflow, declared};
use crate::executor::Executor;
use crate::model::{Dispatching, Holding, Model, ModelError};
use crate::record::{Behaviour, Recorder, Recorders, RecordingFactory};
use crate::value::{Takes, Typed};
use crate::world::World;

/// One journey the scenario ran.
#[derive(Debug)]
pub(crate) struct Journey {
    /// The journey ID its workflow instance produced.
    pub(crate) journey_id: JourneyId,
    /// The recorders of the reporters its instance made, by the names the workflow lists.
    pub(crate) reporters: BTreeMap<String, Recorder>,
    /// What `run` returned.
    pub(crate) run: Result<JourneyResult, Refusal>,
}

/// Why the runner could not run the scenario's workflow.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Unrunnable {
    #[error("the case is in error: {0}")]
    Model(#[from] ModelError),
    #[error("the workflow instance could not be created: {0}")]
    Instance(#[from] InstanceError),
    #[error("the scenario has no executor to run under")]
    NoExecutor,
}

/// Runs the scenario's workflow this many times, each with a new instance, under one executor
/// of the scenario's kind, given the dispatchers the scenario says. A workflow refused as it is
/// built runs no journey.
pub(crate) async fn run(world: &mut World, times: usize) -> Result<(), Unrunnable> {
    let executor = world.executor.ok_or(Unrunnable::NoExecutor)?;
    let factory = recording_factory(&world.model, &mut world.recorders);
    match (executor, factory) {
        (Executor::Local, Some(factory)) => {
            let executor = LocalExecutor::with_dispatcher_factory(factory);
            journeys::<Synchronous, _>(world, times, executor).await
        }
        (Executor::Local, None) => {
            journeys::<Synchronous, _>(world, times, LocalExecutor::new()).await
        }
        (Executor::AsyncLocal, Some(factory)) => {
            let executor = AsyncLocalExecutor::with_dispatcher_factory(factory);
            journeys::<Asynchronous, _>(world, times, executor).await
        }
        (Executor::AsyncLocal, None) => {
            journeys::<Asynchronous, _>(world, times, AsyncLocalExecutor::new()).await
        }
    }
}

/// The factory of the dispatchers the scenario gives the executor, or none when the executor
/// uses its default dispatcher.
fn recording_factory(model: &Model, recorders: &mut Recorders) -> Option<RecordingFactory> {
    match &model.dispatching {
        Dispatching::Unstated => Some(RecordingFactory::new(
            recorders.own(),
            Behaviour::Makes(Holding::AddsReporters),
        )),
        Dispatching::Holding {
            reporter,
            behaviour,
        } => Some(RecordingFactory::new(
            recorders.named(reporter, model),
            Behaviour::Makes(*behaviour),
        )),
        Dispatching::FailingFactory => Some(RecordingFactory::new(
            recorders.own(),
            Behaviour::FailsToCreate,
        )),
        Dispatching::Default => None,
    }
}

/// An executor of the mode's workflows.
trait Runs<M: Mode> {
    async fn run(
        &mut self,
        instance: Instance<ScriptedWorkflow, M>,
    ) -> Result<JourneyResult, Refusal>;
}

impl<F: DispatcherFactory> Runs<Synchronous> for LocalExecutor<F> {
    async fn run(
        &mut self,
        instance: Instance<ScriptedWorkflow, Synchronous>,
    ) -> Result<JourneyResult, Refusal> {
        LocalExecutor::run(self, instance)
    }
}

impl<F: AsyncDispatcherFactory> Runs<Asynchronous> for AsyncLocalExecutor<F> {
    async fn run(
        &mut self,
        instance: Instance<ScriptedWorkflow, Asynchronous>,
    ) -> Result<JourneyResult, Refusal> {
        AsyncLocalExecutor::run(self, instance).await
    }
}

async fn journeys<M: Declares, E: Runs<M>>(
    world: &mut World,
    times: usize,
    mut executor: E,
) -> Result<(), Unrunnable> {
    for _ in 0..times {
        let Some(journey) = journey(world, &mut executor).await? else {
            return Ok(());
        };
        world.journeys.push(journey);
    }
    Ok(())
}

/// Declares and builds the workflow, then runs one journey of a new instance, unless the
/// workflow is refused.
///
/// Each journey has a workflow declared anew, so that its steps count their attempts from the
/// first.
async fn journey<M: Declares, E: Runs<M>>(
    world: &mut World,
    executor: &mut E,
) -> Result<Option<Journey>, Unrunnable> {
    let descriptor = match declared::<M>(&world.model, &world.steps_run)?.build() {
        Ok(descriptor) => descriptor,
        Err(violations) => {
            world.admission = Some(Admission::Refused(violations));
            return Ok(None);
        }
    };
    world.admission = Some(Admission::Admitted(descriptor.listing()));
    let names = &world.model.workflow()?.reporters;
    let recorders: Vec<Recorder> = names
        .iter()
        .map(|name| world.recorders.renewed(name, &world.model))
        .collect();
    let reporters = names
        .iter()
        .cloned()
        .zip(recorders.iter().cloned())
        .collect();
    let instance = descriptor.instance(ScriptedWorkflow::with_reporters(recorders));
    let instance = world
        .model
        .initial_data
        .iter()
        .try_fold(instance, with_entry)?
        .create()?;
    let journey_id = instance.journey_id().clone();
    let run = executor.run(instance).await;
    Ok(Some(Journey {
        journey_id,
        reporters,
        run,
    }))
}

/// The instance builder, with this entry of the initial data.
fn with_entry<M: Mode>(
    builder: InstanceBuilder<ScriptedWorkflow, M>,
    (key, value): &(String, Value),
) -> Result<InstanceBuilder<ScriptedWorkflow, M>, ModelError> {
    Ok(Typed::of(value)?.given_to(Entering { builder, key }))
}

/// Puts a value in the initial data.
struct Entering<'k, M: Mode> {
    builder: InstanceBuilder<ScriptedWorkflow, M>,
    key: &'k str,
}

impl<M: Mode> Takes for Entering<'_, M> {
    type Output = InstanceBuilder<ScriptedWorkflow, M>;

    fn take<T: Storable>(self, value: T) -> Self::Output {
        self.builder.data(self.key, value)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use itinera::event::Event;

    use super::*;
    use crate::model::{Attempt, AttemptOutcome, Reason};
    use crate::trace::Line;

    /// The events the sentences about the whole stream read, from one journey of a workflow with
    /// these steps, scripted by `script` and run under the executor, on a runtime like the
    /// runner's own, whose steps may block on their script.
    pub(crate) fn ran(executor: Executor, steps: &[&str], script: fn(&mut Model)) -> Vec<Event> {
        let mut world = World {
            executor: Some(executor),
            ..World::default()
        };
        let steps = steps.iter().copied().map(str::to_owned).collect();
        world.model.declare("orders".to_owned(), steps).unwrap();
        script(&mut world.model);
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(run(&mut world, 1))
            .unwrap();
        world.recorders.stream(&world.model).unwrap().events()
    }

    pub(crate) fn attempt(outcome: AttemptOutcome) -> Attempt {
        Attempt {
            outcome,
            contributes: Vec::new(),
        }
    }

    pub(crate) fn timed_out() -> AttemptOutcome {
        AttemptOutcome::Failure {
            reason: Reason {
                code: "timeout".to_owned(),
                message: None,
                details: None,
            },
            retriable: true,
        }
    }

    /// The charge step times out once, then succeeds, contributing a receipt.
    fn retried(model: &mut Model) {
        let charge = model.step_mut("charge").unwrap();
        charge.retries = 1;
        charge.attempts = Some(vec![
            attempt(timed_out()),
            Attempt {
                outcome: AttemptOutcome::Success,
                contributes: vec![("receipt".to_owned(), serde_json::json!("R-1"))],
            },
        ]);
    }

    fn lines(events: &[Event]) -> Vec<Line> {
        events.iter().map(Line::from).collect()
    }

    #[test]
    fn both_executors_run_a_scripted_workflow_to_the_same_events() {
        let local = ran(Executor::Local, &["charge"], retried);
        let asynchronous = ran(Executor::AsyncLocal, &["charge"], retried);
        let kinds: Vec<&str> = local.iter().map(Event::kind).collect();
        assert_eq!(
            kinds,
            [
                "journey_started",
                "attempt_started",
                "step_failed",
                "step_retrying",
                "attempt_started",
                "step_succeeded",
                "contribution_committed",
                "journey_succeeded"
            ]
        );
        assert_eq!(lines(&local), lines(&asynchronous));
    }
}
