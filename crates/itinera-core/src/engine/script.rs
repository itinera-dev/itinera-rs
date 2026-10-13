//! The script's runner, in a test's scripted run: each attempt the script plays has its step
//! built, then dropped unrun, and does what the script says instead. Every other attempt runs its
//! step.

use super::runner::{Attempt, Ran, Runner, Steps, lend};
use crate::error::{Error, Interrupted};
use crate::journey::Contributed;
use crate::step::{Action, Outcome, Played, Reporting, Script};

impl Runner for Script {
    fn run<'a>(&'a self, attempt: Attempt<'a>) -> Ran<'a> {
        match self.played(attempt.building.attempt).cloned() {
            None => Steps.run(attempt),
            Some(played) => Box::pin(play(attempt, played)),
        }
    }
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
    contributions: &mut Contributed,
    mut reporting: Reporting<'_>,
) -> Result<Outcome, Error> {
    for action in played.actions {
        act(action, contributions, &mut reporting).await?;
    }
    played.ending.outcome()
}

async fn act(
    action: Action,
    contributions: &mut Contributed,
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
