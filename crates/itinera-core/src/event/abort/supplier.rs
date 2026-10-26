//! The custom code that failed to supply what a step is built from, as events tell it.

use crate::service::ProviderName;
use crate::workflow::AdapterName;

/// The custom code that failed to supply what a step is built from: the input adapter resolving
/// one of its inputs, or the service provider for one of its services.
///
/// # Examples
///
/// ```
/// use itinera::event::Supplier;
///
/// fn key(supplier: &Supplier) -> Option<&str> {
///     match supplier {
///         Supplier::Adapter { key, .. } => Some(key),
///         _ => None,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Supplier {
    /// An input adapter, for one of the step's inputs.
    #[non_exhaustive]
    Adapter {
        /// The adapter's name.
        adapter: AdapterName,
        /// The key of the input it was resolving.
        key: String,
    },
    /// A service provider, for one of the step's services.
    #[non_exhaustive]
    Provider {
        /// The provider's name.
        provider: ProviderName,
    },
}
