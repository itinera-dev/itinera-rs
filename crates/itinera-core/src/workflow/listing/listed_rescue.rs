//! The rescue section of a step's entry in a workflow's listing.

use super::ListedService;
use crate::step::StepName;
use crate::workflow::AdapterName;

/// The rescue section of a step's entry in a workflow's listing: its rescue step, with the input
/// adapter and the providers that serve it. It has no position, since the rescue step runs only
/// in its step's place, and no policies, since a rescue step has none.
///
/// # Examples
///
/// ```
/// use itinera::workflow::ListedRescue;
///
/// fn describe(listed: &ListedRescue) -> String {
///     format!("{}, requesting {} services", listed.step, listed.services.len())
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListedRescue {
    /// The rescue step's name: its step's, followed by `_rescue`.
    pub step: StepName,
    /// The input adapter attached to the rescue step's name, if any.
    pub adapter: Option<AdapterName>,
    /// The services the rescue step requests, in the order it declares them.
    pub services: Vec<ListedService>,
}
