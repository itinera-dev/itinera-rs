//! A service a step requests, in a workflow's listing.

use crate::service::ProviderName;

/// A service a step requests, in a workflow's listing.
///
/// # Examples
///
/// ```
/// use itinera::workflow::ListedService;
///
/// fn describe(listed: &ListedService) -> String {
///     format!("{}: {}", listed.key, listed.provider)
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListedService {
    /// The key the step requests the service by.
    pub key: &'static str,
    /// The provider that serves the step for that key.
    pub provider: ProviderName,
}

pub(super) fn served_by(key: &'static str, provider: ProviderName) -> ListedService {
    ListedService { key, provider }
}
