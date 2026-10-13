//! Services: objects of operations that steps use to do their business, and the service providers
//! through which a workflow supplies them, for a journey or for each attempt.

use std::any::Any;

#[cfg(feature = "async")]
mod asynchronous;
mod descriptor;
mod fit;
#[cfg(test)]
pub(crate) mod fixtures;
mod implementation;
mod provided;
mod scope;
mod token;

#[cfg(feature = "async")]
pub use asynchronous::AsyncProvide;
pub use descriptor::ServiceProviderDescriptor;
pub(crate) use descriptor::{ProviderEntry, serving};
#[doc(hidden)]
pub use fit::{Char, ProvidesService};
pub use implementation::Implementation;
pub use provided::{Provided, provide};
pub(crate) use scope::Scope;
pub use scope::{PerAttempt, PerJourney, ProviderScope};
pub use token::Service;
pub(crate) use token::{ServiceType, lent};

/// A service as the executor keeps it, whatever its type: the `Box<T>` holding the instance a
/// provider made.
pub(crate) type Instance = Box<dyn Any + Send + Sync>;

/// What a provider made, as the executor keeps it, whatever the service's type.
pub(crate) type AnyProvided = Provided<dyn Any + Send + Sync>;

/// A service provider's name, unique among the workflow's providers.
///
/// # Examples
///
/// ```
/// use itinera::service::ProviderName;
///
/// let payments = ProviderName::from("payments");
/// let text: &str = payments.as_ref();
/// assert_eq!(text, "payments");
/// assert_eq!(payments.to_string(), "payments");
/// ```
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
)]
#[as_ref(forward)]
pub struct ProviderName(&'static str);
