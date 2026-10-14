//! Running the scenario's workflow with its script: one journey of a new instance, through
//! itinera's test support, on the local executor of the workflow's mode.

use itinera::instance::WorkflowInstance as _;
use itinera::mode::{Asynchronous, Synchronous};

use super::instance::{Created, Reporters, created};
use super::{Journey, Ran, Unrunnable};
use crate::declaration::Declares;
use crate::executor::Executor;
use crate::fixture;
use crate::model::ModelError;
use crate::testing::{PlaysScripts, unless_stopped, written};
use crate::world::World;

/// Runs one journey of a new instance with the scenario's script, in the execution mode of the
/// scenario's executor. That executor has no dispatcher factory from the runner, so the
/// runner's own recorder is the instance's first reporter. A mistake in the script stops the
/// test, and no journey runs.
pub(crate) async fn run_scripted(world: &mut World) -> Result<(), Unrunnable> {
    match world.executor.ok_or(Unrunnable::NoExecutor)? {
        Executor::Local => scripted::<Synchronous>(world).await,
        Executor::AsyncLocal => scripted::<Asynchronous>(world).await,
    }
}

async fn scripted<M: Declares + PlaysScripts>(world: &mut World) -> Result<(), Unrunnable> {
    if let Some(fixture) = fixture::named(world)? {
        return Err(ModelError::Unscripted(fixture).into());
    }
    let Some(Created {
        instance,
        reporters,
    }) = created::<M>(world, Reporters::OwnFirst).await?
    else {
        return Ok(());
    };
    let journey_id = instance.journey_id().clone();
    let testing = &world.model.testing;
    let played = unless_stopped(async {
        let written = written(instance, &testing.script, testing.every_step_scripted)?;
        Ok::<_, ModelError>(M::played(written).await)
    })
    .await;
    match played {
        Ok(run) => world.journeys.push(Journey {
            journey_id,
            reporters,
            run: run?.map(Ran::Scripted),
        }),
        Err(stopped) => world.tested.stopped = Some(stopped),
    }
    Ok(())
}
