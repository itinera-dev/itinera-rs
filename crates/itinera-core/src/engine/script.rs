//! The script's runner, in a test's scripted run: each attempt the script plays has its step
//! built, then dropped unrun, and does what the script says instead. Every other attempt runs its
//! step. Either way, it records what the step is built with.

use std::mem;
use std::sync::{Arc, Mutex, PoisonError};

use super::runner::{Attempt, Building, Runner, Steps, lend};
use super::{Delivery, run_with};
use crate::error::{Error, Interrupted};
use crate::executor::Started;
use crate::instance::WorkflowInstance;
use crate::journey::{AttemptRecord, Contributor, ScriptedRun};
use crate::step::{Action, Outcome, Played, Reporting, Script, StepDescriptor, StepName};
use crate::value::AnyValue;

/// Runs the journey of the started instance as [`run`](super::run) does, with each attempt the
/// script plays played instead of running its step, and records what every attempt handed to the
/// construction of its step was given.
pub(crate) async fn run_scripted<I: WorkflowInstance>(
    started: Started<I, impl Delivery>,
    script: Script,
) -> ScriptedRun {
    let names: Vec<StepName> = started
        .instance
        .descriptor()
        .steps()
        .iter()
        .flat_map(StepDescriptor::with_rescue_step)
        .map(StepDescriptor::name)
        .collect();
    let runner = Arc::new(ScriptRunner::new(script));
    let result = run_with(
        started,
        std::time::SystemTime::now,
        Arc::<ScriptRunner>::clone(&runner),
    )
    .await;
    ScriptedRun::new(result, names, &runner.recorded())
}

/// Plays each attempt the script plays and hands every other to [`Steps`], recording what each
/// attempt's step is built with, whether or not its construction succeeds.
struct ScriptRunner {
    script: Script,
    /// A record of each attempt handed to it, in order.
    recorded: Mutex<Vec<AttemptRecord>>,
}

impl ScriptRunner {
    fn new(script: Script) -> Self {
        Self {
            script,
            recorded: Mutex::default(),
        }
    }

    /// The records of the attempts handed to it so far, which it no longer keeps.
    fn recorded(&self) -> Vec<AttemptRecord> {
        mem::take(&mut *self.recorded.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn record(&self, building: &Building<'_>) {
        let record = AttemptRecord::new(building.attempt.clone(), built_with(building));
        self.recorded
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(record);
    }
}

#[async_trait::async_trait]
impl Runner for ScriptRunner {
    async fn run<'a>(&'a self, attempt: Attempt<'a>) -> Result<Result<Outcome, Error>, Error> {
        self.record(&attempt.building);
        match self.script.played(attempt.building.attempt).cloned() {
            None => Steps.run(attempt).await,
            Some(played) => play(attempt, played).await,
        }
    }
}

/// The values the attempt's step is built with: its inputs that have one, then the contributions
/// a rescue step requested from the failed attempt.
fn built_with(building: &Building<'_>) -> Vec<(&'static str, AnyValue)> {
    building
        .inputs
        .iter()
        .cloned()
        .chain(building.rescued.contributed.untaken())
        .filter_map(present)
        .collect()
}

fn present((key, value): (&'static str, Option<AnyValue>)) -> Option<(&'static str, AnyValue)> {
    Some(key).zip(value)
}

/// Builds the attempt's step and drops it, then does what the script says the attempt does, in
/// order, as the step would through its handles, and ends it as the script says. An interrupted
/// emit ends it at once, as a step that propagates it.
async fn play(attempt: Attempt<'_>, played: Played) -> Result<Result<Outcome, Error>, Error> {
    let Attempt {
        factory,
        needs,
        building,
        contributions,
        emitting,
    } = attempt;
    let source = building.attempt.clone();
    factory.build(lend(needs, building, &mut *contributions, &mut *emitting))?;
    Ok(acted(played, contributions, Reporting::new(emitting, source)).await)
}

async fn acted(
    played: Played,
    contributions: &mut Contributor,
    mut reporting: Reporting<'_>,
) -> Result<Outcome, Error> {
    for action in played.actions {
        act(action, contributions, &mut reporting).await?;
    }
    played.ending.outcome()
}

async fn act(
    action: Action,
    contributions: &mut Contributor,
    reporting: &mut Reporting<'_>,
) -> Result<(), Interrupted> {
    match action {
        Action::Contribute { key, value } => {
            contributions.add(key, value);
            Ok(())
        }
        Action::Emit {
            level,
            message,
            data,
        } => reporting.emit(level, message, data).await,
        Action::SetGlobalMetadata { key, value } => reporting.set_global_metadata(key, value).await,
    }
}

#[cfg(test)]
mod tests;
