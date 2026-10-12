//! One step's entry in a workflow's listing.

use std::num::NonZeroUsize;

use super::{ListedRescue, ListedService};
use crate::policy::PolicyName;
use crate::step::StepName;
use crate::workflow::AdapterName;

/// One step of a workflow's listing.
///
/// # Examples
///
/// ```
/// use itinera::workflow::ListedStep;
///
/// fn describe(listed: &ListedStep) -> String {
///     format!("{}. {}", listed.position, listed.step)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListedStep {
    /// The step's name.
    pub step: StepName,
    /// Where the step runs among the workflow's steps, from 1.
    pub position: NonZeroUsize,
    /// The step policies attached to it, in the order they were attached.
    pub policies: Vec<PolicyName>,
    /// The input adapter attached to it, if any.
    pub adapter: Option<AdapterName>,
    /// The services it requests, in the order it declares them.
    pub services: Vec<ListedService>,
    /// Its rescue section, when it has a rescue step.
    pub rescue: Option<ListedRescue>,
}
