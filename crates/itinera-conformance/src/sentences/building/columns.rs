//! The columns of the steps table, each standing for the sentences it names, for its step.

use std::str::FromStr;

use super::super::policies::step_has_the_policies;
use super::super::services::step_requests_the_service;
use super::{ends, request};
use crate::model::{Attempt, ModelError, Row, ServiceType, ValueType};
use crate::sentences::Names;
use crate::world::World;

const COLUMNS: [&str; 7] = [
    "step", "policies", "inputs", "services", "outcome", "code", "rescue",
];

/// Scripts the row's step with what its filled cells stand for, in the order of the columns.
pub(super) fn script(world: &mut World, row: &Row) -> Result<(), ModelError> {
    row.only(&COLUMNS)?;
    let step = row.required("step")?;
    if let Some(policies) = row.optional("policies") {
        step_has_the_policies(world, step.to_owned(), names(policies))?;
    }
    if let Some(inputs) = row.optional("inputs") {
        for (key, value_type) in typed::<ValueType>(inputs, "inputs")? {
            request(world, step, key, value_type, false)?;
        }
    }
    if let Some(services) = row.optional("services") {
        for (key, service_type) in typed::<ServiceType>(services, "services")? {
            step_requests_the_service(world, step.to_owned(), key, service_type)?;
        }
    }
    if let Some(attempt) = attempt(row.optional("outcome"), row.optional("code"))? {
        ends(world, step, vec![attempt])?;
    }
    match row.optional("rescue") {
        Some("yes") => world.model.rescue(step),
        Some(other) => Err(ModelError::Cell("rescue", other.to_owned())),
        None => Ok(()),
    }
}

/// The names of a cell, separated by commas, in the order written.
fn names(cell: &str) -> Names {
    Names::from(cell.split(", ").map(str::to_owned).collect::<Vec<_>>())
}

/// How every attempt of the step ends, as the `outcome` column, and `code`, say.
fn attempt(outcome: Option<&str>, code: Option<&str>) -> Result<Option<Attempt>, ModelError> {
    match (outcome, code) {
        (Some("success"), None) => Ok(Some(Attempt::success())),
        (Some("failure"), Some(code)) => Ok(Some(Attempt::failure(code.to_owned()))),
        (Some("failure"), None) => Err(ModelError::MissingCell("code")),
        (_, Some(code)) => Err(ModelError::Cell("code", code.to_owned())),
        (Some(other), None) => Err(ModelError::Cell("outcome", other.to_owned())),
        (None, None) => Ok(None),
    }
}

/// The requests of a cell in this column, `key: type` separated by commas, in the order written.
fn typed<T: FromStr>(cell: &str, column: &'static str) -> Result<Vec<(String, T)>, ModelError> {
    cell.split(", ")
        .map(|written| request_of(written, column))
        .collect()
}

fn request_of<T: FromStr>(written: &str, column: &'static str) -> Result<(String, T), ModelError> {
    let unreadable = || ModelError::Cell(column, written.to_owned());
    let (key, name) = written.split_once(": ").ok_or_else(unreadable)?;
    let parsed = name.parse().map_err(|_| unreadable())?;
    Ok((key.to_owned(), parsed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AttemptOutcome, Input};

    fn ordering() -> World {
        let mut world = World::default();
        world
            .model
            .declare("orders".to_owned(), vec!["charge".to_owned()])
            .unwrap();
        world
    }

    #[test]
    fn each_filled_cell_scripts_the_step_as_its_sentences_do() {
        let mut world = ordering();
        world
            .model
            .define("audit".to_owned(), "on step success", true)
            .unwrap();
        let cells = [
            ("step", "charge"),
            ("policies", "audit"),
            ("inputs", "amount: integer, card: string"),
            ("services", "usage: meter"),
            ("outcome", "failure"),
            ("code", "declined"),
            ("rescue", "yes"),
        ];

        script(&mut world, &Row::of(&cells)).unwrap();

        let workflow = world.model.workflow().unwrap();
        assert_eq!(workflow.step_policies["charge"], ["audit"]);
        let charge = &workflow.scripts["charge"];
        assert_eq!(
            charge.inputs,
            [
                Input {
                    key: "amount".to_owned(),
                    value_type: ValueType::Integer,
                    optional: false,
                },
                Input {
                    key: "card".to_owned(),
                    value_type: ValueType::String,
                    optional: false,
                },
            ]
        );
        assert_eq!(charge.services, [("usage".to_owned(), ServiceType::Meter)]);
        let attempts = charge.attempts.as_ref().unwrap();
        assert!(matches!(
            &attempts[0].outcome,
            AttemptOutcome::Failure(reason) if reason.code == "declined"
        ));
        assert!(charge.rescued);
        assert!(workflow.scripts["charge_rescue"].rescues);
    }

    #[test]
    fn an_empty_cell_adds_nothing() {
        let mut world = ordering();

        script(&mut world, &Row::of(&[("step", "charge"), ("inputs", "")])).unwrap();

        let charge = &world.model.workflow().unwrap().scripts["charge"];
        assert!(charge.inputs.is_empty());
        assert!(charge.attempts.is_none());
    }

    #[test]
    fn a_request_written_without_its_type_is_refused() {
        let mut world = ordering();

        let scripted = script(
            &mut world,
            &Row::of(&[("step", "charge"), ("inputs", "amount")]),
        );

        assert_eq!(
            scripted,
            Err(ModelError::Cell("inputs", "amount".to_owned()))
        );
    }
}
