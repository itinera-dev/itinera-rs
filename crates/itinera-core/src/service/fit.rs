//! The bound through which the macros check, as the program compiles, that a step or a rescue
//! step requests each service as the type its workflow's providers make. It is hidden: an
//! application declaring services with the builder never names it, and `build()` checks every
//! type either way.

/// One character of a service's key, as a type: a key is the nested tuple of its characters,
/// ending with `()`, such as `(Char<'p'>, (Char<'a'>, ()))` for `"pa"`, so that the step's
/// expansion and the workflow's both name the same type for the same key.
///
/// # Examples
///
/// ```
/// use itinera::service::Char;
///
/// type Pa = (Char<'p'>, (Char<'a'>, ()));
/// let _key: Pa = (Char, (Char, ()));
/// ```
#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct Char<const C: char>;

/// That the workflow's providers of the service of key `K` make it as `T`: implemented for the
/// one type every one of them returns, or for every type when they return types written apart,
/// in which case `build()` alone checks the steps and rescue steps they serve.
///
/// # Examples
///
/// ```
/// use itinera::service::{Char, ProvidesService};
///
/// trait StockStore: Send + Sync {}
///
/// struct Orders;
///
/// impl ProvidesService<(Char<'s'>, ()), dyn StockStore> for Orders {}
///
/// fn makes<W: ProvidesService<(Char<'s'>, ()), dyn StockStore>>() {}
/// makes::<Orders>();
/// ```
///
/// A step requesting the service as another type does not fit:
///
/// ```compile_fail
/// use itinera::service::{Char, ProvidesService};
///
/// trait StockStore: Send + Sync {}
/// trait PaymentGateway: Send + Sync {}
///
/// struct Orders;
///
/// impl ProvidesService<(Char<'s'>, ()), dyn StockStore> for Orders {}
///
/// fn makes<W: ProvidesService<(Char<'s'>, ()), dyn PaymentGateway>>() {}
/// makes::<Orders>();
/// ```
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "a step or rescue step of the workflow `{Self}` requests a service as `{T}`, which \
               no provider of its key makes",
    label = "the step or rescue step listed here requests a service as `{T}`",
    note = "a provider of the key returns `Result<Provided<T>, Error>` for the `T` that the \
            `#[service]` parameter borrows, in a step's `run` or `new` or a rescue step's `run`; \
            when no provider has the key, the error `service not provided` names it"
)]
pub trait ProvidesService<K, T: ?Sized> {}
