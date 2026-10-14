//! Running the scenario's workflow: its journeys, one after another, under the executor of the
//! scenario's copy.

#[cfg(test)]
pub(crate) mod fixtures;
mod initial;
mod instance;
mod runs;
mod scripted;

use std::collections::BTreeMap;

use itinera::error::Error;
use itinera::executor::{AsyncLocalExecutor, LocalExecutor, Refusal};
use itinera::instance::{InstanceError, WorkflowInstance as _};
use itinera::journey::{JourneyId, JourneyResult, ScriptedRun};
use itinera::mode::{Asynchronous, Synchronous};

pub(crate) use runs::Runs;
pub(crate) use scripted::run_scripted;

use crate::declaration::{Admission, Declares};
use crate::executor::Executor;
use crate::fixture::{self, Fixture, RunsFixtures};
use crate::model::{Dispatching, Holding, Model, ModelError};
use crate::record::{Behaviour, Recorder, Recorders, RecordingFactory};
use crate::world::World;
use instance::{Created, Reporters, created};

/// One journey the scenario ran.
#[derive(Debug)]
pub(crate) struct Journey {
    /// The journey ID its workflow instance produced.
    pub(crate) journey_id: JourneyId,
    /// The recorders of the reporters its instance made, by the names the workflow lists.
    pub(crate) reporters: BTreeMap<String, Recorder>,
    /// What `run` returned.
    pub(crate) run: Result<Ran, Refusal>,
}

impl Journey {
    /// The journey's result, unless `run` was refused.
    pub(crate) fn result(&self) -> Result<&JourneyResult, &Refusal> {
        self.run.as_ref().map(Ran::result)
    }
}

/// What a journey's run gave: its result, or a scripted run, with its result and the record of
/// its attempts.
#[derive(Debug)]
pub(crate) enum Ran {
    Journey(JourneyResult),
    Scripted(ScriptedRun),
}

impl Ran {
    pub(crate) fn result(&self) -> &JourneyResult {
        match self {
            Self::Journey(result) => result,
            Self::Scripted(run) => &run.result,
        }
    }

    /// The record of a scripted run's attempts.
    pub(crate) fn record(&self) -> Option<&ScriptedRun> {
        match self {
            Self::Journey(_) => None,
            Self::Scripted(run) => Some(run),
        }
    }
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
    #[error("the {0} workflow is not declared in this executor's execution mode")]
    OtherMode(Fixture),
    #[error("what the scenario runs alone could not run: {0}")]
    Alone(Error),
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

async fn journeys<M: Declares + RunsFixtures, E: Runs<M>>(
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
/// workflow is refused or test support stops the test.
///
/// Each journey has a workflow declared anew, so that its steps count their attempts from the
/// first.
async fn journey<M: Declares + RunsFixtures, E: Runs<M>>(
    world: &mut World,
    executor: &mut E,
) -> Result<Option<Journey>, Unrunnable> {
    if let Some(fixture) = fixture::named(world)? {
        world.admission = Some(Admission::Admitted(fixture.listing()));
        return M::journey(fixture, &world.model, executor).await.map(Some);
    }
    let Some(Created {
        instance,
        reporters,
    }) = created::<M>(world, Reporters::Listed).await?
    else {
        return Ok(None);
    };
    let journey_id = instance.journey_id().clone();
    let run = executor.run(instance).await.map(Ran::Journey);
    Ok(Some(Journey {
        journey_id,
        reporters,
        run,
    }))
}

#[cfg(test)]
mod tests {
    use itinera::event::Event;

    use super::*;
    use crate::journey::fixtures::{attempt, lines, ran, timed_out};
    use crate::model::{Attempt, AttemptOutcome};

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
