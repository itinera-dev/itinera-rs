//! The script of a scripted run, written row by row in itinera's own words, and the run itself
//! on the local executor of the workflow's mode.

mod acts;

use std::iter;

use itinera::executor::Refusal;
use itinera::instance::{
    AttemptScripting, Named, Opened, Scripted, StepScripting, WorkflowInstance,
};
use itinera::journey::ScriptedRun;
use itinera::mode::{Asynchronous, Mode, Synchronous};

use crate::model::{Chosen, ModelError, Played, ScriptRow, ScriptedEnding};
use acts::{act, actions};

/// A script as far as it is written: started, or describing a step.
pub(crate) enum Written<I: WorkflowInstance> {
    Started(Scripted<I>),
    Step(StepScripting<I, I::Mode>),
}

/// The instance's script, written from these rows in order, ensuring every step is scripted
/// if `every`. A mistake in writing it stops the test as it is written.
pub(crate) fn written<I: WorkflowInstance>(
    instance: I,
    rows: &[ScriptRow],
    every: bool,
) -> Result<Written<I>, ModelError> {
    let previous = iter::once(None).chain(rows.iter().map(Some));
    let written = rows
        .iter()
        .zip(previous)
        .try_fold(Written::Started(instance.scripted()), with_row)?;
    Ok(if every {
        written.ensuring_all()
    } else {
        written
    })
}

/// The script, with the row after the one before it, if any.
fn with_row<I: WorkflowInstance>(
    written: Written<I>,
    (row, previous): (&ScriptRow, Option<&ScriptRow>),
) -> Result<Written<I>, ModelError> {
    let opened = match (written, previous) {
        (Written::Step(step), Some(previous)) if previous.step == row.step => {
            continued(step, row.attempts)
        }
        (Written::Started(scripted), _) => started(scripted.step(&row.step), row.attempts)?,
        (Written::Step(step), _) => started(step.and_step(&row.step), row.attempts)?,
    };
    described(opened, &row.plays).map(Written::Step)
}

impl<I: WorkflowInstance> Written<I> {
    fn ensuring_all(self) -> Self {
        match self {
            Self::Started(scripted) => Self::Started(scripted.ensure_all_scripted()),
            Self::Step(step) => Self::Step(step.ensure_all_scripted()),
        }
    }
}

/// The attempts the first row of a step chooses; `remaining` never starts a step's rows.
fn started<I: WorkflowInstance>(
    named: AttemptScripting<I, I::Mode, Named>,
    attempts: Chosen,
) -> Result<AttemptScripting<I, I::Mode, Opened>, ModelError> {
    match attempts {
        Chosen::Number(number) => Ok(named.on_attempt(number)),
        Chosen::Every => Ok(named.on_every_attempt()),
        Chosen::Remaining => Err(ModelError::Cell("attempt", "remaining".to_owned())),
    }
}

/// The attempts a later row of the same step chooses.
fn continued<I: WorkflowInstance>(
    step: StepScripting<I, I::Mode>,
    attempts: Chosen,
) -> AttemptScripting<I, I::Mode, Opened> {
    match attempts {
        Chosen::Number(number) => step.on_attempt(number),
        Chosen::Every => step.on_every_attempt(),
        Chosen::Remaining => step.on_remaining_attempts(),
    }
}

/// What a row's attempts do, then how they end.
fn described<I: WorkflowInstance>(
    opened: AttemptScripting<I, I::Mode, Opened>,
    plays: &Played,
) -> Result<StepScripting<I, I::Mode>, ModelError> {
    let (acts, ending) = match plays {
        Played::ForReal => return Ok(opened.will_run_for_real()),
        Played::Scripted { acts, ending } => (acts, ending),
    };
    let mut actions = actions(acts)?.into_iter();
    Ok(match actions.next() {
        None => ended(opened, ending),
        Some(first) => ended(actions.fold(act(opened, first), act), ending),
    })
}

fn ended<I: WorkflowInstance, S>(
    scripting: AttemptScripting<I, I::Mode, S>,
    ending: &ScriptedEnding,
) -> StepScripting<I, I::Mode> {
    match ending {
        ScriptedEnding::Success => scripting.will_succeed(),
        ScriptedEnding::Failure(code) => scripting.will_fail(code.as_str()),
        ScriptedEnding::Skipped(None) => scripting.will_skip(),
        ScriptedEnding::Skipped(Some(code)) => scripting.will_skip_because(code.as_str()),
        ScriptedEnding::AbnormalTermination(message) => {
            scripting.will_terminate_abnormally(message.as_str())
        }
    }
}

/// An execution mode, whose local executor runs a scripted run of its workflows.
pub(crate) trait PlaysScripts: Mode + Sized {
    async fn played<I: WorkflowInstance<Mode = Self>>(
        written: Written<I>,
    ) -> Result<ScriptedRun, Refusal>;
}

impl PlaysScripts for Synchronous {
    async fn played<I: WorkflowInstance<Mode = Self>>(
        written: Written<I>,
    ) -> Result<ScriptedRun, Refusal> {
        match written {
            Written::Started(scripted) => scripted.run(),
            Written::Step(step) => step.run(),
        }
    }
}

impl PlaysScripts for Asynchronous {
    async fn played<I: WorkflowInstance<Mode = Self>>(
        written: Written<I>,
    ) -> Result<ScriptedRun, Refusal> {
        match written {
            Written::Started(scripted) => scripted.run().await,
            Written::Step(step) => step.run().await,
        }
    }
}
