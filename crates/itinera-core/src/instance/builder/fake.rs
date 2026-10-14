//! Giving an instance fakes in place of its workflow's service providers, for a test.

use super::InstanceBuilder;
use crate::error::Error;
use crate::instance::FakeProvider;
use crate::mode::Mode;
use crate::service::{Fake, Provided, ProviderName};

impl<W: Send + Sync + 'static, M: Mode> InstanceBuilder<W, M> {
    /// Puts `fake` in place of the function of the workflow's service provider named `name`, for
    /// this instance alone, so that the steps it serves run against an instance the test makes,
    /// replacing any fake given before for that provider. The provider keeps its scope, the
    /// steps it is attached to and what it requests: the executor still resolves its requests
    /// first, so missing data aborts the journey as without a fake, then calls `fake` wherever
    /// it would call the provider, once per journey or once per attempt. An error from `fake`
    /// aborts the journey as the provider's would. `service_provided` names the implementation
    /// `fake` made.
    ///
    /// The provider is named by text, so the service's type `T` must be named too, as in
    /// `fake_provider::<dyn StockStore>`.
    ///
    /// It is available with the `testing` feature, which an application enables under
    /// `[dev-dependencies]`.
    ///
    /// # Panics
    ///
    /// When the workflow has no service provider named `name`, or when `T` is not the type of
    /// service the provider supplies.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::executor::LocalExecutor;
    /// use itinera::report::RecordingReporter;
    /// use itinera::service::{Service, ServiceProviderDescriptor, provide};
    /// use itinera::step::{
    ///     Outcome, Reason, Resolved, Step, StepDescriptor, StepFactory, StepNeeds, step_name,
    /// };
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// trait StockStore: Send + Sync {
    ///     fn in_stock(&self) -> bool;
    /// }
    ///
    /// struct Warehouse;
    ///
    /// impl StockStore for Warehouse {
    ///     fn in_stock(&self) -> bool {
    ///         true
    ///     }
    /// }
    ///
    /// struct SoldOut;
    ///
    /// impl StockStore for SoldOut {
    ///     fn in_stock(&self) -> bool {
    ///         false
    ///     }
    /// }
    ///
    /// const STOCK: Service<dyn StockStore> = Service::new("stock");
    ///
    /// struct Reserve<'a> {
    ///     stock: &'a dyn StockStore,
    /// }
    ///
    /// impl Step for Reserve<'_> {
    ///     fn run(self) -> Result<Outcome, Error> {
    ///         if self.stock.in_stock() {
    ///             Ok(Outcome::success())
    ///         } else {
    ///             Ok(Outcome::failure(Reason::new("sold-out")))
    ///         }
    ///     }
    /// }
    ///
    /// struct Reserving;
    ///
    /// impl StepFactory for Reserving {
    ///     type Step<'a> = Reserve<'a>;
    ///
    ///     fn needs(&self) -> StepNeeds {
    ///         StepNeeds::new().service(&STOCK)
    ///     }
    ///
    ///     fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Reserve<'a>, Error> {
    ///         Ok(Reserve { stock: got.service(&STOCK)? })
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("reserve"), Reserving))
    ///     .provider(ServiceProviderDescriptor::new("stock", &STOCK, |_, _| {
    ///         Ok(provide!(Warehouse))
    ///     }))
    ///     .build()?;
    /// let recorder = RecordingReporter::default();
    /// let instance = orders
    ///     .instance(Orders)
    ///     .reporter(recorder.clone())
    ///     .fake_provider::<dyn StockStore>("stock", || Ok(provide!(SoldOut)))
    ///     .create()?;
    /// LocalExecutor::new().run(instance)?;
    /// assert_eq!(
    ///     recorder.path(),
    ///     [
    ///         "reserve attempt 1 failed: sold-out",
    ///         "reserve given up: failure",
    ///         "journey failed at reserve: sold-out",
    ///     ]
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[track_caller]
    pub fn fake_provider<T: ?Sized + Send + Sync + 'static>(
        mut self,
        name: impl Into<ProviderName>,
        fake: impl Fn() -> Result<Provided<T>, Error> + Send + Sync + 'static,
    ) -> Self {
        self.descriptor.fake_provider(name.into(), Fake::new(fake));
        self
    }
}

impl<W: Send + Sync + 'static, M: Mode, T: ?Sized + Send + Sync + 'static> FakeProvider<T>
    for InstanceBuilder<W, M>
{
    #[track_caller]
    fn __itinera_fake_provider(
        self,
        name: ProviderName,
        fake: impl Fn() -> Result<Provided<T>, Error> + Send + Sync + 'static,
    ) -> Self {
        self.fake_provider(name, fake)
    }
}
