//! The implementation a service provider made: the name of its instance's type.

use std::any;

/// The implementation a service provider made: the name of the instance's type, as Rust
/// describes it, such as `orders::Bank`. It is the instance's concrete type when the provider
/// made it with [`provide!`], and the type the provider declares, such as `dyn PaymentGateway`,
/// when it gave [`Provided::from_box`] an instance already boxed. Its format is the compiler's own
/// and may change from one version to the next, so reporters should not depend on it.
///
/// [`provide!`]: crate::service::provide
/// [`Provided::from_box`]: crate::service::Provided::from_box
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
/// let implementation = stock.implementation();
/// let name: &str = implementation.as_ref();
/// assert!(name.ends_with("Warehouse"));
/// assert_eq!(implementation.to_string(), name);
/// ```
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, derive_more::Display, derive_more::Into, derive_more::AsRef,
)]
#[as_ref(forward)]
pub struct Implementation(&'static str);

impl Implementation {
    /// What [`provide!`] expands to: the implementation that `value` is, by the name of its type.
    /// Applications write [`provide!`] instead.
    ///
    /// [`provide!`]: crate::service::provide
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::Implementation;
    ///
    /// struct Warehouse;
    ///
    /// let name: &str = Implementation::__of(&Warehouse).into();
    /// assert!(name.ends_with("Warehouse"));
    /// ```
    #[doc(hidden)]
    pub fn __of<V>(value: &V) -> Self {
        Self(any::type_name_of_val(value))
    }

    /// The implementation named by the type `T`.
    pub(crate) fn of_type<T: ?Sized>() -> Self {
        Self(any::type_name::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait PaymentGateway {}

    struct Bank;

    impl PaymentGateway for Bank {}

    #[test]
    fn a_value_is_the_implementation_its_concrete_type_names() {
        let gateway: &dyn PaymentGateway = &Bank;

        assert_eq!(
            Implementation::__of(&Bank),
            Implementation::of_type::<Bank>()
        );
        assert_ne!(
            Implementation::__of(&gateway),
            Implementation::of_type::<Bank>()
        );
    }
}
