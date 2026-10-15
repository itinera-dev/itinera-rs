//! The service providers of asynchronous workflows, whose instances are awaited.

use std::future::Future;

use super::descriptor::Provide;
use super::{
    PerAttempt, PerJourney, Provided, ProviderName, ProviderScope, Service,
    ServiceProviderDescriptor,
};
use crate::error::Error;
use crate::mode::Asynchronous;
use crate::policy::{Provider, Requested};

/// The function of a service provider of an asynchronous workflow: given the workflow's own value
/// and what the provider requested, it makes an instance of the service `T` once awaited. An
/// `async fn` of the workflow taking `&self` and a [`Requested`] is one.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{Provider, Requested};
/// use itinera::service::{AsyncProvide, PerJourney, Provided, provide};
///
/// trait StockStore: Send + Sync {}
///
/// struct Warehouse;
///
/// impl StockStore for Warehouse {}
///
/// struct Orders;
///
/// impl Orders {
///     async fn stock(
///         &self,
///         _got: Requested<'_, Self, Provider<PerJourney>, Asynchronous>,
///     ) -> Result<Provided<dyn StockStore>> {
///         Ok(provide!(Warehouse))
///     }
/// }
///
/// fn provides_stock<P: for<'a> AsyncProvide<'a, Orders, PerJourney, dyn StockStore>>(_: P) {}
///
/// provides_stock(Orders::stock);
/// ```
pub trait AsyncProvide<'a, W, S: ProviderScope, T: ?Sized>: Send + Sync + 'static {
    /// The provider's call, which ends with the instance it made.
    type Future: Future<Output = Result<Provided<T>, Error>> + Send + 'a;

    /// Calls the provider.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{Provider, Requested};
    /// use itinera::service::{AsyncProvide, PerJourney, Provided};
    ///
    /// trait StockStore: Send + Sync {}
    ///
    /// async fn stock<'a, W, P: AsyncProvide<'a, W, PerJourney, dyn StockStore>>(
    ///     provider: &P,
    ///     workflow: &'a W,
    ///     got: Requested<'a, W, Provider<PerJourney>, Asynchronous>,
    /// ) -> Result<Provided<dyn StockStore>> {
    ///     provider.provide(workflow, got).await
    /// }
    /// ```
    fn provide(
        &self,
        workflow: &'a W,
        got: Requested<'a, W, Provider<S>, Asynchronous>,
    ) -> Self::Future;
}

impl<'a, W: 'a, S, T, F, R> AsyncProvide<'a, W, S, T> for F
where
    S: ProviderScope,
    T: ?Sized,
    F: Fn(&'a W, Requested<'a, W, Provider<S>, Asynchronous>) -> R + Send + Sync + 'static,
    R: Future<Output = Result<Provided<T>, Error>> + Send + 'a,
{
    type Future = R;

    fn provide(&self, workflow: &'a W, got: Requested<'a, W, Provider<S>, Asynchronous>) -> R {
        self(workflow, got)
    }
}

impl<W> ServiceProviderDescriptor<W, PerJourney, Asynchronous> {
    /// Describes a provider of scope [`PerJourney`] of an asynchronous workflow, with its name,
    /// the service it supplies and its function, attached to no step. It needs nothing, until
    /// [`needing`](Self::needing) says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{Provider, Requested};
    /// use itinera::service::{PerJourney, Provided, Service, ServiceProviderDescriptor, provide};
    ///
    /// trait StockStore: Send + Sync {}
    ///
    /// struct Warehouse;
    ///
    /// impl StockStore for Warehouse {}
    ///
    /// const STOCK: Service<dyn StockStore> = Service::new("stock");
    ///
    /// struct Orders;
    ///
    /// impl Orders {
    ///     async fn stock(
    ///         &self,
    ///         _got: Requested<'_, Self, Provider<PerJourney>, Asynchronous>,
    ///     ) -> Result<Provided<dyn StockStore>> {
    ///         Ok(provide!(Warehouse))
    ///     }
    /// }
    ///
    /// let stock = ServiceProviderDescriptor::new_async("stock", &STOCK, Orders::stock);
    /// ```
    pub fn new_async<T: ?Sized + Send + Sync + 'static>(
        name: impl Into<ProviderName>,
        service: &Service<T>,
        provide: impl for<'a> AsyncProvide<'a, W, PerJourney, T>,
    ) -> Self {
        Self::holding(name.into(), service, awaited(provide))
    }
}

impl<W> ServiceProviderDescriptor<W, PerAttempt, Asynchronous> {
    /// Describes a provider of scope [`PerAttempt`] of an asynchronous workflow, with its name,
    /// the service it supplies and its function, attached to no step. It needs nothing, until
    /// [`needing`](Self::needing) says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{Provider, Requested};
    /// use itinera::service::{PerAttempt, Provided, Service, ServiceProviderDescriptor, provide};
    ///
    /// trait PaymentGateway: Send + Sync {}
    ///
    /// struct Main;
    ///
    /// impl PaymentGateway for Main {}
    ///
    /// const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");
    ///
    /// struct Orders;
    ///
    /// impl Orders {
    ///     async fn payments(
    ///         &self,
    ///         _got: Requested<'_, Self, Provider<PerAttempt>, Asynchronous>,
    ///     ) -> Result<Provided<dyn PaymentGateway>> {
    ///         Ok(provide!(Main))
    ///     }
    /// }
    ///
    /// let payments = ServiceProviderDescriptor::per_attempt_async("payments", &PAYMENTS, Orders::payments);
    /// ```
    pub fn per_attempt_async<T: ?Sized + Send + Sync + 'static>(
        name: impl Into<ProviderName>,
        service: &Service<T>,
        provide: impl for<'a> AsyncProvide<'a, W, PerAttempt, T>,
    ) -> Self {
        Self::holding(name.into(), service, awaited(provide))
    }
}

/// An asynchronous provider's function, as the executor calls it.
fn awaited<W, S: ProviderScope, T: ?Sized + Send + Sync + 'static>(
    provide: impl for<'a> AsyncProvide<'a, W, S, T>,
) -> Box<Provide<W, S, Asynchronous>> {
    Box::new(move |workflow, got| {
        let provided = provide.provide(workflow, got);
        Box::pin(async move { provided.await.map(Provided::erased) })
    })
}
