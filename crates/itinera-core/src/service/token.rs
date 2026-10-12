//! A service as steps request it and providers supply it: a key and a type.

use std::any::{self, TypeId};
use std::marker::PhantomData;

use super::Instance;

/// A service: a key, unique among the services of a step, and the type `T` of what is lent to the
/// step, a trait object such as `dyn PaymentGateway`, so that the step depends on what the service
/// can do rather than on what provides it. `#[service]` refuses any other type; a step written
/// with the builder keeps to a trait object too, since a token cannot refuse one.
///
/// A step declares it in its [`StepNeeds`] and takes it from [`Resolved`] when it is built; a
/// [`ServiceProviderDescriptor`] supplies it. A service is not a value: it never enters the data
/// bag, an event or the result.
///
/// [`StepNeeds`]: crate::step::StepNeeds
/// [`Resolved`]: crate::step::Resolved
/// [`ServiceProviderDescriptor`]: super::ServiceProviderDescriptor
///
/// # Examples
///
/// ```
/// use itinera::service::Service;
///
/// trait PaymentGateway: Send + Sync {
///     fn charge(&self, amount: i64) -> String;
/// }
///
/// const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");
/// assert_eq!(PAYMENTS.key(), "payments");
/// ```
#[derive(derive_more::Debug)]
pub struct Service<T: ?Sized> {
    key: &'static str,
    #[debug(skip)]
    service: PhantomData<fn() -> Box<T>>,
}

impl<T: ?Sized> Service<T> {
    /// Declares a service under this key.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::Service;
    ///
    /// trait StockStore: Send + Sync {}
    ///
    /// let stock = Service::<dyn StockStore>::new("stock");
    /// ```
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            service: PhantomData,
        }
    }

    /// The service's key.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::Service;
    ///
    /// trait StockStore: Send + Sync {}
    ///
    /// assert_eq!(Service::<dyn StockStore>::new("stock").key(), "stock");
    /// ```
    pub fn key(&self) -> &'static str {
        self.key
    }
}

/// The type of a service, compared when a provider is checked against the steps it serves.
#[derive(Clone, Copy, PartialEq, Eq, derive_more::Debug)]
#[debug("{name}")]
pub(crate) struct ServiceType {
    id: TypeId,
    name: &'static str,
}

impl ServiceType {
    /// The type of the services `T`, lent as `&T`.
    pub(crate) fn of<T: ?Sized + Send + Sync + 'static>() -> Self {
        Self {
            id: TypeId::of::<Box<T>>(),
            name: any::type_name::<T>(),
        }
    }
}

/// The service of type `T` that an instance holds, if it holds one of that type.
pub(crate) fn lent<T: ?Sized + 'static>(instance: &Instance) -> Option<&T> {
    instance.downcast_ref::<Box<T>>().map(AsRef::as_ref)
}
