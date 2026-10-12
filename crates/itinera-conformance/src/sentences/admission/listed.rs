//! A workflow's listing, as the cases write it: one line for each step, with the optional
//! columns the table has.

use std::num::NonZeroUsize;

use itinera::workflow::{ListedRescue, ListedService, ListedStep};

use crate::model::{ModelError, Row};

/// Which optional columns a listing's table has. One it does not have is not compared.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Columns {
    services: bool,
    rescue_adapter: bool,
    rescue_services: bool,
}

impl Columns {
    /// The optional columns of the table this row belongs to.
    pub(super) fn of(row: &Row) -> Self {
        Self {
            services: has(row, "services"),
            rescue_adapter: has(row, "rescue adapter"),
            rescue_services: has(row, "rescue services"),
        }
    }
}

fn has(row: &Row, column: &str) -> bool {
    row.columns().any(|name| is_column(name, column))
}

fn is_column(name: &str, column: &str) -> bool {
    name == column
}

/// One line of a listing, as the cases write it. A cell of an optional column is `None` when the
/// table does not have that column.
#[derive(Debug, PartialEq)]
pub(super) struct Listed {
    step: String,
    position: NonZeroUsize,
    policies: String,
    adapter: String,
    rescue: String,
    services: Option<String>,
    rescue_adapter: Option<String>,
    rescue_services: Option<String>,
}

impl Listed {
    pub(super) fn expected(row: &Row, columns: Columns) -> Result<Self, ModelError> {
        Ok(Self {
            step: row.required("step")?.to_owned(),
            position: row.parse("position")?,
            policies: cell(row, "policies"),
            adapter: cell(row, "adapter"),
            rescue: cell(row, "rescue"),
            services: columns.services.then(|| cell(row, "services")),
            rescue_adapter: columns.rescue_adapter.then(|| cell(row, "rescue adapter")),
            rescue_services: columns
                .rescue_services
                .then(|| cell(row, "rescue services")),
        })
    }

    pub(super) fn found(listed: &ListedStep, columns: Columns) -> Self {
        let rescue = listed.rescue.as_ref();
        let rescue_adapter = rescue.and_then(rescue_adapter).unwrap_or_default();
        let rescue_services = rescue.map(rescue_services).unwrap_or_default();
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
            rescue: rescue.map(rescue_step).unwrap_or_default(),
            services: columns.services.then(|| services_of(&listed.services)),
            rescue_adapter: columns.rescue_adapter.then_some(rescue_adapter),
            rescue_services: columns.rescue_services.then_some(rescue_services),
        }
    }
}

fn rescue_step(rescue: &ListedRescue) -> String {
    rescue.step.to_string()
}

fn rescue_adapter(rescue: &ListedRescue) -> Option<String> {
    rescue.adapter.as_ref().map(ToString::to_string)
}

fn rescue_services(rescue: &ListedRescue) -> String {
    services_of(&rescue.services)
}

/// Services in the listing, as the cases write them, separated by commas.
fn services_of(services: &[ListedService]) -> String {
    services
        .iter()
        .map(served_by)
        .collect::<Vec<_>>()
        .join(", ")
}

/// A service in the listing, as the cases write it: its key and the provider that serves it.
fn served_by(service: &ListedService) -> String {
    format!("{}: {}", service.key, service.provider)
}

/// The text of a cell, empty when the row leaves it empty.
fn cell(row: &Row, column: &'static str) -> String {
    row.optional(column).unwrap_or_default().to_owned()
}
