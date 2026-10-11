//! What the engine resolved for one attempt, before a factory builds the step from it.

use crate::journey::Contributor;
use crate::step::{FromRescued, Reporting, Slots};
use crate::value::AnyValue;

/// What the engine resolved for one attempt, whatever the workflow's mode.
#[derive(derive_more::Debug)]
pub(crate) struct Got<'a> {
    inputs: Slots,
    pub(super) rescued: FromRescued,
    pub(super) contributor: Option<Contributor<'a>>,
    pub(super) reporting: Option<Reporting<'a>>,
}

impl<'a> Got<'a> {
    /// Holds the values of the inputs, in the order the needs declare them, absent ones as
    /// `None`, what a rescue step requested from the failed attempt, and the handles the needs
    /// declare.
    pub(crate) fn new(
        inputs: impl IntoIterator<Item = (&'static str, Option<AnyValue>)>,
        rescued: FromRescued,
        contributor: Option<Contributor<'a>>,
        reporting: Option<Reporting<'a>>,
    ) -> Self {
        Self {
            inputs: inputs.into_iter().collect(),
            rescued,
            contributor,
            reporting,
        }
    }

    pub(super) fn take(&mut self, key: &str) -> Option<AnyValue> {
        self.inputs.take(key)
    }

    pub(super) fn take_optional(&mut self, key: &str) -> Option<Option<AnyValue>> {
        self.inputs.take_optional(key)
    }
}
