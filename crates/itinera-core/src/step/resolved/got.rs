//! What the engine resolved for one attempt, before a factory builds the step from it.

use crate::journey::Contributor;
#[cfg(feature = "unstable")]
use crate::service::Instance;
use crate::step::{FromRescued, Reporting, Slots};
use crate::value::AnyValue;

/// What the engine resolved for one attempt, whatever the workflow's mode.
#[derive(derive_more::Debug)]
pub(crate) struct Got<'a> {
    inputs: Slots,
    pub(super) rescued: FromRescued,
    #[cfg(feature = "unstable")]
    #[debug(skip)]
    services: Vec<(&'static str, &'a Instance)>,
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
            #[cfg(feature = "unstable")]
            services: Vec::new(),
            contributor,
            reporting,
        }
    }

    /// Lends the step the instances of the services it declares, by key.
    #[cfg(feature = "unstable")]
    pub(crate) fn lending(mut self, services: Vec<(&'static str, &'a Instance)>) -> Self {
        self.services = services;
        self
    }

    #[cfg(feature = "unstable")]
    pub(super) fn lent(&self, key: &str) -> Option<&'a Instance> {
        self.services
            .iter()
            .find(|lent| is_lent_under(lent, key))
            .map(instance_of)
    }

    pub(super) fn take(&mut self, key: &str) -> Option<AnyValue> {
        self.inputs.take(key)
    }

    pub(super) fn take_optional(&mut self, key: &str) -> Option<Option<AnyValue>> {
        self.inputs.take_optional(key)
    }
}

#[cfg(feature = "unstable")]
fn is_lent_under((lent, _): &&(&'static str, &Instance), key: &str) -> bool {
    *lent == key
}

#[cfg(feature = "unstable")]
fn instance_of<'a>((_, instance): &(&'static str, &'a Instance)) -> &'a Instance {
    instance
}
