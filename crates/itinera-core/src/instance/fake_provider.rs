//! The bound through which the builder `#[workflow]` generates gives a fake in place of a service
//! provider. It is hidden: a test calls `fake_provider` on that builder, or on an
//! [`InstanceBuilder`](crate::instance::InstanceBuilder).

use crate::error::Error;
use crate::service::{Provided, ProviderName};

/// That an instance builder takes a fake supplying the service `T` in place of a service
/// provider's function, which only the `testing` feature implements. The builder `#[workflow]`
/// generates requires it of its `fake_provider`, so a test calling that method without the
/// feature gets a compile error naming it, while a build that never calls it compiles either way.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::instance::{FakeProvider, InstanceBuilder};
/// use itinera::service::{Provided, provide};
///
/// trait StockStore: Send + Sync {}
///
/// struct Shelf;
///
/// impl StockStore for Shelf {}
///
/// fn shelf() -> Result<Provided<dyn StockStore>> {
///     Ok(provide!(Shelf))
/// }
///
/// fn faked<W>(builder: InstanceBuilder<W>) -> InstanceBuilder<W>
/// where
///     InstanceBuilder<W>: FakeProvider<dyn StockStore>,
/// {
///     builder.__itinera_fake_provider("stock".into(), shelf)
/// }
/// ```
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`fake_provider` needs itinera's `testing` feature",
    label = "a fake in place of a service provider",
    note = "enable the `testing` feature of itinera under `[dev-dependencies]`"
)]
pub trait FakeProvider<T: ?Sized> {
    /// The builder, with `fake` in place of the function of the provider named `name`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::instance::{FakeProvider, InstanceBuilder};
    /// use itinera::service::{Provided, provide};
    ///
    /// trait StockStore: Send + Sync {}
    ///
    /// struct Shelf;
    ///
    /// impl StockStore for Shelf {}
    ///
    /// fn shelf() -> Result<Provided<dyn StockStore>> {
    ///     Ok(provide!(Shelf))
    /// }
    ///
    /// fn faked<W>(builder: InstanceBuilder<W>) -> InstanceBuilder<W>
    /// where
    ///     InstanceBuilder<W>: FakeProvider<dyn StockStore>,
    /// {
    ///     builder.__itinera_fake_provider("stock".into(), shelf)
    /// }
    /// ```
    fn __itinera_fake_provider(
        self,
        name: ProviderName,
        fake: impl Fn() -> Result<Provided<T>, Error> + Send + Sync + 'static,
    ) -> Self;
}
