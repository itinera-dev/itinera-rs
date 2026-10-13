//! What a service provider returns: the instance it made, with the name of its implementation
//! and the data it supplies with it.

use super::{AnyProvided, Implementation, Instance};
use crate::value::{AnyValue, Value};

mod provide;

pub use provide::provide;

/// What a service provider returns for the service `T`: the instance it made, the
/// [`Implementation`] that instance is, and the data the provider supplies with it, if any, which
/// `service_provided` carries for the reporters. The step receives only the instance, as `&T`.
///
/// It is made with [`provide!`], which reads the name of the instance's concrete type before the
/// instance becomes a `T`, such as a `dyn PaymentGateway`, which hides it. An instance that is
/// already a `Box<T>` is made into one with [`Provided::from_box`], which can only name `T`.
///
/// [`provide!`]: crate::service::provide
///
/// # Examples
///
/// ```
/// use itinera::service::{Provided, provide};
///
/// trait PaymentGateway: Send + Sync {
///     fn charge(&self, amount: i64) -> String;
/// }
///
/// struct Sandbox;
///
/// impl PaymentGateway for Sandbox {
///     fn charge(&self, amount: i64) -> String {
///         format!("charged {amount} in the sandbox")
///     }
/// }
///
/// let payments: Provided<dyn PaymentGateway> = provide!(Sandbox);
/// assert!(payments.implementation().to_string().ends_with("Sandbox"));
/// assert_eq!(payments.service().charge(5), "charged 5 in the sandbox");
/// ```
#[derive(derive_more::Debug)]
pub struct Provided<T: ?Sized> {
    implementation: Implementation,
    #[debug(skip)]
    service: Box<T>,
    data: Option<AnyValue>,
}

impl<T: ?Sized> Provided<T> {
    /// What [`provide!`] expands to: the instance, already boxed as `T`, with the implementation
    /// it is and the provider's data. Applications write [`provide!`] instead.
    ///
    /// [`provide!`]: crate::service::provide
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::{Implementation, Provided};
    ///
    /// trait StockStore: Send + Sync {}
    ///
    /// struct Warehouse;
    ///
    /// impl StockStore for Warehouse {}
    ///
    /// let warehouse = Warehouse;
    /// let implementation = Implementation::__of(&warehouse);
    /// let stock: Provided<dyn StockStore> = Provided::__new(implementation, Box::new(warehouse), None);
    /// assert!(stock.implementation().to_string().ends_with("Warehouse"));
    /// ```
    #[doc(hidden)]
    pub fn __new(implementation: Implementation, service: Box<T>, data: Option<AnyValue>) -> Self {
        Self {
            implementation,
            service,
            data,
        }
    }

    /// An instance that is already a `Box<T>`, such as one a library made. The box hides the
    /// instance's concrete type, so its implementation is `T`, the type the provider declares,
    /// such as `dyn PaymentGateway`, and reporters cannot tell one implementation from another.
    /// Prefer [`provide!`], which names the concrete type, whenever the provider has the instance
    /// itself. Otherwise, the provider should say which implementation it made in the data it
    /// supplies with [`Provided::with_data`], such as the name of the bank it connects to.
    ///
    /// [`provide!`]: crate::service::provide
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::Provided;
    ///
    /// trait PaymentGateway: Send + Sync {}
    ///
    /// struct Sandbox;
    ///
    /// impl PaymentGateway for Sandbox {}
    ///
    /// fn from_a_library() -> Box<dyn PaymentGateway> {
    ///     Box::new(Sandbox)
    /// }
    ///
    /// let payments = Provided::from_box(from_a_library()).with_data("the sandbox bank".to_string());
    /// assert!(payments.implementation().to_string().contains("dyn"));
    /// assert!(payments.data().is_some());
    /// ```
    pub fn from_box(service: Box<T>) -> Self {
        Self::__new(Implementation::of_type::<T>(), service, None)
    }

    /// The same instance, with `data` supplied with it, which must be a [`Value`] and which
    /// `service_provided` carries for the reporters. It replaces any data supplied before.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::{Provided, provide};
    ///
    /// trait StockStore: Send + Sync {}
    ///
    /// struct Warehouse;
    ///
    /// impl StockStore for Warehouse {}
    ///
    /// let stock: Provided<dyn StockStore> = provide!(Warehouse);
    /// let north = stock.with_data("north".to_string());
    /// let data = north.data().and_then(|data| data.downcast_ref::<String>());
    /// assert_eq!(data.map(String::as_str), Some("north"));
    /// ```
    #[must_use]
    pub fn with_data<V: Value>(self, data: V) -> Self {
        Self {
            data: Some(AnyValue::new(data)),
            ..self
        }
    }

    /// The implementation the instance is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::{Provided, provide};
    ///
    /// trait StockStore: Send + Sync {}
    ///
    /// struct Warehouse;
    ///
    /// impl StockStore for Warehouse {}
    ///
    /// let stock: Provided<dyn StockStore> = provide!(Warehouse);
    /// assert!(stock.implementation().to_string().ends_with("Warehouse"));
    /// ```
    pub fn implementation(&self) -> Implementation {
        self.implementation
    }

    /// The instance, as the step receives it.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::{Provided, provide};
    ///
    /// trait StockStore: Send + Sync {
    ///     fn count(&self, item: &str) -> i64;
    /// }
    ///
    /// struct Warehouse;
    ///
    /// impl StockStore for Warehouse {
    ///     fn count(&self, _item: &str) -> i64 {
    ///         3
    ///     }
    /// }
    ///
    /// let stock: Provided<dyn StockStore> = provide!(Warehouse);
    /// assert_eq!(stock.service().count("book"), 3);
    /// ```
    pub fn service(&self) -> &T {
        &self.service
    }

    /// The data the provider supplied with the instance, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::{Provided, provide};
    ///
    /// trait StockStore: Send + Sync {}
    ///
    /// struct Warehouse;
    ///
    /// impl StockStore for Warehouse {}
    ///
    /// let plain: Provided<dyn StockStore> = provide!(Warehouse);
    /// assert!(plain.data().is_none());
    ///
    /// let described: Provided<dyn StockStore> = provide!(Warehouse, "north".to_string());
    /// let north = described.data().and_then(|data| data.downcast_ref::<String>());
    /// assert_eq!(north.map(String::as_str), Some("north"));
    /// ```
    pub fn data(&self) -> Option<&AnyValue> {
        self.data.as_ref()
    }
}

impl<T: ?Sized + Send + Sync + 'static> Provided<T> {
    /// The instance as the executor keeps it, whatever its type, with its implementation and the
    /// provider's data.
    pub(crate) fn erased(self) -> AnyProvided {
        Provided {
            implementation: self.implementation,
            service: Box::new(self.service),
            data: self.data,
        }
    }
}

impl AnyProvided {
    /// The instance the executor lends, its implementation and the provider's data, apart.
    pub(crate) fn into_parts(self) -> (Instance, Implementation, Option<AnyValue>) {
        (self.service, self.implementation, self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::lent;

    trait PaymentGateway: Send + Sync {}

    struct Bank;

    impl PaymentGateway for Bank {}

    fn bank() -> Provided<dyn PaymentGateway> {
        provide!(Bank)
    }

    #[test]
    fn the_implementation_is_the_concrete_type_of_the_instance_not_the_service_type() {
        let made = bank();

        assert_eq!(made.implementation(), Implementation::of_type::<Bank>());
    }

    #[test]
    fn an_instance_already_boxed_is_named_by_the_type_it_is_boxed_as() {
        let boxed: Box<dyn PaymentGateway> = Box::new(Bank);

        assert_eq!(
            Provided::from_box(boxed).implementation(),
            Implementation::of_type::<dyn PaymentGateway>()
        );
    }

    #[test]
    fn an_erased_instance_is_still_lent_as_the_service_type() {
        let (instance, implementation, data) = bank().erased().into_parts();

        assert!(lent::<dyn PaymentGateway>(&instance).is_some());
        assert_eq!(implementation, Implementation::of_type::<Bank>());
        assert!(data.is_none());
    }
}
