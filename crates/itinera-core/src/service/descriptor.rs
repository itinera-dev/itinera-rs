//! Service provider descriptors: a provider's name, the service it supplies, its scope, the steps
//! it is attached to, what it needs and its function.

use std::future;

use super::{
    AnyProvided, PerAttempt, PerJourney, Provided, Provider, ProviderName, ProviderScope, Service,
    ServiceType,
};
use crate::error::Error;
use crate::mode::Synchronous;
use crate::policy::{HookCall, HookNeeds, Needs, Requested};
use crate::step::StepName;

mod entry;

pub(crate) use entry::{ProviderEntry, serving};

/// The description of a service provider of workflows `W`, of scope `S`, in mode `M`: its name,
/// the [`Service`] it supplies, the steps it is attached to, if any, what it needs, and its
/// function. A provider is a hook of the workflow itself, of the kind [`Provider`].
///
/// It is declared on the workflow with [`WorkflowBuilder::provider`]. It serves the steps it is
/// attached to that request its service, or, attached to no step, every step that requests its
/// service and has no provider of its own for it. It is called while a step it serves is built,
/// after the step's inputs, with the workflow's own value and what it [`Requested`]: the journey
/// ID, the data from the workflow it declared, read from the data bag, a reporter if it declared
/// one, and, for a provider of scope [`PerAttempt`], the step's name and the attempt number.
///
/// - Of scope [`PerJourney`], it is called the first time an attempt of a step it serves needs
///   its service, and that instance is lent to every later attempt of every step it serves,
///   until the journey ends.
/// - Of scope [`PerAttempt`], it is called for every attempt of every step it serves, and each
///   instance is lent to that attempt only.
///
/// An error aborts the journey with `step could not be built`. An instance is dropped once its
/// scope ends: when its attempt ends, or when the journey ends.
///
/// [`WorkflowBuilder::provider`]: crate::workflow::WorkflowBuilder::provider
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::policy::Requested;
/// use itinera::service::{
///     PerAttempt, Provided, Provider, Service, ServiceProviderDescriptor, provide,
/// };
/// use itinera::step::step_name;
///
/// trait PaymentGateway: Send + Sync {}
///
/// struct Main;
/// struct Backup;
///
/// impl PaymentGateway for Main {}
/// impl PaymentGateway for Backup {}
///
/// const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");
///
/// struct Orders;
///
/// impl Orders {
///     fn payments(
///         &self,
///         got: Requested<'_, Self, Provider<PerAttempt>>,
///     ) -> Result<Provided<dyn PaymentGateway>> {
///         if got.attempt().is_first() {
///             Ok(provide!(Main))
///         } else {
///             Ok(provide!(Backup))
///         }
///     }
/// }
///
/// let payments = ServiceProviderDescriptor::per_attempt("payments", &PAYMENTS, Orders::payments)
///     .step(step_name!("charge"));
/// assert_eq!(payments.name().to_string(), "payments");
/// ```
#[derive(derive_more::Debug)]
pub struct ServiceProviderDescriptor<W, S: ProviderScope, M = Synchronous> {
    name: ProviderName,
    key: &'static str,
    service_type: ServiceType,
    steps: Vec<StepName>,
    needs: Needs,
    #[debug(skip)]
    provide: Box<Provide<W, S, M>>,
}

/// How the executor calls a provider, which ends with what it made.
pub(super) type Provide<W, S, M> = dyn for<'a> Fn(&'a W, Requested<'a, W, Provider<S>, M>) -> HookCall<'a, AnyProvided>
    + Send
    + Sync;

impl<W> ServiceProviderDescriptor<W, PerJourney> {
    /// Describes a provider of scope [`PerJourney`] with its name, the service it supplies and
    /// its function, attached to no step. It needs nothing, until
    /// [`needing`](Self::needing) says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::{Service, ServiceProviderDescriptor, provide};
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
    /// let stock = ServiceProviderDescriptor::new("stock", &STOCK, |_: &Orders, _| {
    ///     Ok(provide!(Warehouse))
    /// });
    /// ```
    pub fn new<T: ?Sized + Send + Sync + 'static>(
        name: impl Into<ProviderName>,
        service: &Service<T>,
        provide: impl for<'a> Fn(
            &'a W,
            Requested<'a, W, Provider<PerJourney>>,
        ) -> Result<Provided<T>, Error>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self::holding(name.into(), service, synchronously(provide))
    }
}

impl<W> ServiceProviderDescriptor<W, PerAttempt> {
    /// Describes a provider of scope [`PerAttempt`] with its name, the service it supplies and
    /// its function, attached to no step. It needs nothing, until
    /// [`needing`](Self::needing) says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::{Service, ServiceProviderDescriptor, provide};
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
    /// let payments = ServiceProviderDescriptor::per_attempt("payments", &PAYMENTS, |_: &Orders, _| {
    ///     Ok(provide!(Main))
    /// });
    /// ```
    pub fn per_attempt<T: ?Sized + Send + Sync + 'static>(
        name: impl Into<ProviderName>,
        service: &Service<T>,
        provide: impl for<'a> Fn(
            &'a W,
            Requested<'a, W, Provider<PerAttempt>>,
        ) -> Result<Provided<T>, Error>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self::holding(name.into(), service, synchronously(provide))
    }
}

impl<W, S: ProviderScope, M> ServiceProviderDescriptor<W, S, M> {
    pub(super) fn holding<T: ?Sized + Send + Sync + 'static>(
        name: ProviderName,
        service: &Service<T>,
        provide: Box<Provide<W, S, M>>,
    ) -> Self {
        Self {
            name,
            key: service.key(),
            service_type: ServiceType::of::<T>(),
            steps: Vec::new(),
            needs: Needs::default(),
            provide,
        }
    }

    /// Declares what the provider needs, resolved before each of its calls: data from the
    /// workflow, and a reporter.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::policy::{HookNeeds, Requested};
    /// use itinera::service::{
    ///     PerAttempt, Provided, Provider, Service, ServiceProviderDescriptor, provide,
    /// };
    /// use itinera::step::Input;
    ///
    /// trait PaymentGateway: Send + Sync {}
    ///
    /// struct Gateway(String);
    ///
    /// impl PaymentGateway for Gateway {}
    ///
    /// const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");
    /// const GATEWAY: Input<String> = Input::new("gateway");
    ///
    /// struct Orders;
    ///
    /// impl Orders {
    ///     fn payments(
    ///         &self,
    ///         mut got: Requested<'_, Self, Provider<PerAttempt>>,
    ///     ) -> Result<Provided<dyn PaymentGateway>> {
    ///         Ok(provide!(Gateway(got.from_workflow(&GATEWAY)?)))
    ///     }
    /// }
    ///
    /// let payments = ServiceProviderDescriptor::per_attempt("payments", &PAYMENTS, Orders::payments)
    ///     .needing(HookNeeds::new().from_workflow(&GATEWAY).reporter());
    /// ```
    pub fn needing(mut self, needs: HookNeeds<Provider<S>>) -> Self {
        self.needs = needs.into();
        self
    }

    /// Attaches the provider to a step, or to a rescue step by its name, such as `charge_rescue`,
    /// which it then serves instead of a provider of the same service attached to no step.
    /// Attached to a step twice, the provider provides its service to it twice, so the workflow is
    /// refused with [`Violation::ServiceProvidedTwice`](crate::workflow::Violation::ServiceProvidedTwice).
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::{Service, ServiceProviderDescriptor, provide};
    /// use itinera::step::step_name;
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
    /// let shipping_stock = ServiceProviderDescriptor::new("shipping stock", &STOCK, |_: &Orders, _| {
    ///     Ok(provide!(Warehouse))
    /// })
    /// .step(step_name!("reserve"))
    /// .step(step_name!("ship"));
    /// ```
    pub fn step(mut self, step: StepName) -> Self {
        self.steps.push(step);
        self
    }

    /// The provider's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::service::{Service, ServiceProviderDescriptor, provide};
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
    /// let stock = ServiceProviderDescriptor::new("stock", &STOCK, |_: &Orders, _| {
    ///     Ok(provide!(Warehouse))
    /// });
    /// assert_eq!(stock.name().to_string(), "stock");
    /// ```
    pub fn name(&self) -> ProviderName {
        self.name
    }
}

/// A synchronous provider's function, as the executor calls it: its instance is ready at once.
fn synchronously<W, S: ProviderScope, T: ?Sized + Send + Sync + 'static>(
    provide: impl for<'a> Fn(&'a W, Requested<'a, W, Provider<S>>) -> Result<Provided<T>, Error>
    + Send
    + Sync
    + 'static,
) -> Box<Provide<W, S, Synchronous>> {
    Box::new(move |workflow, got| {
        Box::pin(future::ready(provide(workflow, got).map(Provided::erased)))
    })
}
