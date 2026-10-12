//! The actions: admitting, listing and running the workflow, and what admission and the listing
//! give.

use cucumber::gherkin::Step as Sentence;
use cucumber::{then, when};
use itinera::mode::Synchronous;
use itinera::workflow::Violation;

use super::{Unmet, holds};
use crate::declaration::{Admission, declared};
use crate::fixture;
use crate::journey::{self, Unrunnable};
use crate::model::{ModelError, Row, rows};
use crate::world::World;
use listed::{Columns, Listed};

mod listed;

#[when(expr = "the workflow is admitted")]
fn the_workflow_is_admitted(world: &mut World) -> Result<(), ModelError> {
    admit(world)
}

#[when(expr = "the workflow runs")]
async fn the_workflow_runs(world: &mut World) -> Result<(), Unrunnable> {
    journey::run(world, 1).await
}

#[when(expr = "the workflow is listed")]
fn the_workflow_is_listed(world: &mut World) -> Result<(), ModelError> {
    admit(world)
}

/// Builds the scenario's workflow, which checks its declaration and runs nothing. Neither
/// admission nor the listing depends on the execution mode, so the workflow is declared
/// synchronous. A workflow the macros declare is built already, by its descriptor.
fn admit(world: &mut World) -> Result<(), ModelError> {
    if let Some(fixture) = fixture::named(world)? {
        world.admission = Some(Admission::Admitted(fixture.listing()));
        return Ok(());
    }
    let admission =
        match declared::<Synchronous>(&world.model, &world.steps_run, &world.witness)?.build() {
            Ok(descriptor) => Admission::Admitted(descriptor.listing()),
            Err(violations) => Admission::Refused(violations),
        };
    world.admission = Some(admission);
    Ok(())
}

#[then(expr = "admission is refused with the violations:")]
fn admission_is_refused_with_the_violations(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let mut expected = rows(sentence)?
        .iter()
        .map(violation_name)
        .collect::<Result<Vec<_>, _>>()?;
    expected.sort();
    let mut found: Vec<String> = match admission(world)? {
        Admission::Refused(violations) => violations.iter().map(kind_name).collect(),
        Admission::Admitted(_) => {
            return Err(Unmet::Expected(format!(
                "admission refused with {expected:?}, but the workflow was admitted"
            )));
        }
    };
    found.sort();
    holds(
        found == expected,
        format_args!("admission refused with {expected:?}"),
        format_args!("{found:?}"),
    )
}

fn violation_name(row: &Row) -> Result<String, ModelError> {
    row.required("violation").map(str::to_owned)
}

fn kind_name(violation: &Violation) -> String {
    violation.kind().to_string()
}

#[then(expr = "the listing is:")]
fn the_listing_is(world: &mut World, #[step] sentence: &Sentence) -> Result<(), Unmet> {
    let rows = rows(sentence)?;
    let columns = rows.first().map(Columns::of).unwrap_or_default();
    let expected = rows
        .iter()
        .map(|row| Listed::expected(row, columns))
        .collect::<Result<Vec<_>, _>>()?;
    let found: Vec<Listed> = match admission(world)? {
        Admission::Admitted(listing) => listing
            .iter()
            .map(|listed| Listed::found(listed, columns))
            .collect(),
        Admission::Refused(violations) => {
            return Err(Unmet::Expected(format!(
                "a listing, but admission was refused: {violations}"
            )));
        }
    };
    holds(
        found == expected,
        format_args!("the listing {expected:?}"),
        format_args!("{found:?}"),
    )
}

#[then(expr = "no step ran")]
fn no_step_ran(world: &mut World) -> Result<(), Unmet> {
    holds(world.steps_run()?.none(), "no step to run", "a step ran")
}

fn admission(world: &World) -> Result<&Admission, Unmet> {
    world
        .admission
        .as_ref()
        .ok_or_else(|| Unmet::Case("the workflow was neither admitted nor listed".to_owned()))
}
