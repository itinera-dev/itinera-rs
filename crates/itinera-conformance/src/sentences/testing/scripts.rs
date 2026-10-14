//! Scripted runs: the script, its run, and the mistakes in a test that stop it.

use cucumber::gherkin::Step as Sentence;
use cucumber::{given, then, when};

use crate::journey::{self, Unrunnable};
use crate::model::{ModelError, ScriptRow, rows};
use crate::sentences::{Unmet, holds};
use crate::testing::{Mistake, mistakes};
use crate::world::World;

#[given(expr = "the script says:")]
fn the_script_says(world: &mut World, #[step] sentence: &Sentence) -> Result<(), ModelError> {
    let script = rows(sentence)?
        .iter()
        .map(ScriptRow::of)
        .collect::<Result<Vec<_>, _>>()?;
    world.model.testing.script.extend(script);
    Ok(())
}

#[given(expr = "the script ensures every step is scripted")]
fn the_script_ensures_every_step_is_scripted(world: &mut World) {
    world.model.testing.every_step_scripted = true;
}

#[given(expr = "step {string} declares no contributor and no reporter")]
fn step_declares_no_contributor_and_no_reporter(
    world: &mut World,
    step_name: String,
) -> Result<(), ModelError> {
    world.model.step_mut(&step_name)?.undeclared_handles = true;
    Ok(())
}

#[when(expr = "the workflow runs with the script")]
async fn the_workflow_runs_with_the_script(world: &mut World) -> Result<(), Unrunnable> {
    journey::run_scripted(world).await
}

#[then(expr = "the test stopped before the journey, naming:")]
fn the_test_stopped_before_the_journey(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let mut expected = rows(sentence)?
        .iter()
        .map(Mistake::of)
        .collect::<Result<Vec<_>, _>>()?;
    expected.sort();
    let stopped = world
        .tested
        .stopped
        .as_deref()
        .ok_or_else(|| not_stopped(&expected))?;
    let mut named = mistakes(stopped).ok_or_else(|| stopped_otherwise(&expected, stopped))?;
    named.sort();
    let received = world.recorders.received();
    holds(
        named == expected && received == 0,
        format_args!("the test to stop before the journey, naming {expected:?}"),
        format_args!("{named:?}, from \"{stopped}\", after {received} events"),
    )
}

fn not_stopped(expected: &[Mistake]) -> Unmet {
    Unmet::Expected(format!(
        "the test to stop before the journey, naming {expected:?}; it did not stop"
    ))
}

fn stopped_otherwise(expected: &[Mistake], stopped: &str) -> Unmet {
    Unmet::Expected(format!(
        "the test to stop, naming {expected:?}; it stopped with \"{stopped}\""
    ))
}
