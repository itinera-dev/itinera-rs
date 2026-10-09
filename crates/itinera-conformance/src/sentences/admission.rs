//! Admission and the listing: what the workflow's declaration gives before any journey.

use std::fmt;
use std::num::NonZeroUsize;

use cucumber::gherkin::Step as Sentence;
use cucumber::{then, when};
use itinera::workflow::{ListedStep, Violation};

use super::Unmet;
use crate::declaration::{Admission, declared};
use crate::model::{ModelError, Row, rows};
use crate::world::World;

#[when(expr = "the workflow is admitted")]
fn the_workflow_is_admitted(world: &mut World) -> Result<(), ModelError> {
    admit(world)
}

#[when(expr = "the workflow is listed")]
fn the_workflow_is_listed(world: &mut World) -> Result<(), ModelError> {
    admit(world)
}

/// Builds the scenario's workflow, which checks its declaration and runs nothing.
fn admit(world: &mut World) -> Result<(), ModelError> {
    let admission = match declared(&world.model, &world.steps_run)?.build() {
        Ok(descriptor) => Admission::Admitted(descriptor),
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
    let expected = rows(sentence)?
        .iter()
        .map(Listed::expected)
        .collect::<Result<Vec<_>, _>>()?;
    let found: Vec<Listed> = match admission(world)? {
        Admission::Admitted(descriptor) => descriptor.listing().iter().map(Listed::from).collect(),
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
    holds(world.steps_run.none(), "no step to run", "a step ran")
}

fn admission(world: &World) -> Result<&Admission, Unmet> {
    world
        .admission
        .as_ref()
        .ok_or_else(|| Unmet::Case("the workflow was neither admitted nor listed".to_owned()))
}

/// One line of a listing, as the cases write it.
#[derive(Debug, PartialEq)]
struct Listed {
    step: String,
    position: NonZeroUsize,
    policies: String,
    adapter: String,
}

impl Listed {
    fn expected(row: &Row) -> Result<Self, ModelError> {
        Ok(Self {
            step: row.required("step")?.to_owned(),
            position: row.parse("position")?,
            policies: cell(row, "policies"),
            adapter: cell(row, "adapter"),
        })
    }
}

impl From<&ListedStep> for Listed {
    fn from(listed: &ListedStep) -> Self {
        Self {
            step: listed.step.to_string(),
            position: listed.position,
            policies: listed
                .policies
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            adapter: listed
                .adapter
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
        }
    }
}

/// The text of a cell, empty when the row leaves it empty.
fn cell(row: &Row, column: &'static str) -> String {
    row.optional(column).unwrap_or_default().to_owned()
}

/// Holds when the condition does, and otherwise says what was expected and what was found.
fn holds(
    condition: bool,
    expected: impl fmt::Display,
    found: impl fmt::Display,
) -> Result<(), Unmet> {
    if condition {
        Ok(())
    } else {
        Err(Unmet::Expected(format!(
            "expected {expected}; found {found}"
        )))
    }
}
