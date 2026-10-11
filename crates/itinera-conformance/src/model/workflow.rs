//! The workflow a scenario declares, and how its instance gets its journey ID.

use std::collections::BTreeMap;

use super::{Adapter, Role, Step};

/// The workflow a scenario declares.
#[derive(Debug)]
pub(crate) struct Workflow {
    pub(crate) name: String,
    /// The step names in declaration order, repeated if the scenario declares a name twice.
    pub(crate) steps: Vec<String>,
    /// What each step does, by name, so a repeated name has one script.
    pub(crate) scripts: BTreeMap<String, Step>,
    /// The step policies attached to each step, in order.
    pub(crate) step_policies: BTreeMap<String, Vec<String>>,
    /// The workflow policies attached, in order.
    pub(crate) policies: Vec<String>,
    /// The input adapters, in the order declared.
    pub(crate) adapters: Vec<Adapter>,
    /// The reporters the workflow lists, in order.
    pub(crate) reporters: Vec<String>,
    pub(crate) id_generator: IdGenerator,
    pub(crate) roles: BTreeMap<String, Role>,
}

/// How the workflow instance gets its journey ID.
#[derive(Debug, Default, PartialEq)]
pub(crate) enum IdGenerator {
    /// The default, a UUID version 4.
    #[default]
    Default,
    Returns(String),
}
