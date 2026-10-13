//! Declaring a workflow, one part at a time, and building its descriptor.

use std::sync::Arc;

use super::{
    Declaration, InputAdapterDescriptor, Violations, WorkflowDescriptor, WorkflowName, violation,
};
use crate::error::Error;
use crate::journey::DataBag;
use crate::mode::{Mode, Synchronous};
use crate::policy::WorkflowPolicyDescriptor;
use crate::report::{self, Reporter};
use crate::service::{ProviderScope, ServiceProviderDescriptor};
use crate::step::StepDescriptor;

/// Declares a workflow, one part at a time, and builds its [`WorkflowDescriptor`].
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::journey::DataBag;
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders {
///     prefix: &'static str,
/// }
///
/// fn first_order(orders: &Orders, _: &DataBag) -> Result<String, Error> {
///     Ok(format!("{}-1", orders.prefix))
/// }
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .id_generator(first_order)
///     .build()?;
/// let instance = orders.instance(Orders { prefix: "order" }).create()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(derive_more::Debug)]
pub struct WorkflowBuilder<W, M: Mode = Synchronous> {
    pub(super) declaration: Declaration<W, M>,
}

impl<W: Send + Sync + 'static, M: Mode> WorkflowBuilder<W, M> {
    pub(super) fn named(name: WorkflowName) -> Self {
        Self {
            declaration: Declaration {
                name,
                steps: Vec::new(),
                policies: Vec::new(),
                adapters: Vec::new(),
                providers: Vec::new(),
                reporters: Vec::new(),
                id_generator: None,
            },
        }
    }

    /// Adds a step, after the steps already added. Steps run in the order they are added.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .step(StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success())))
    ///     .build()?;
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn step(mut self, step: StepDescriptor<W, M>) -> Self {
        self.declaration.steps.push(step);
        self
    }

    /// Attaches a workflow policy, after those already attached.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     OnWorkflowSuccess, Requested, WorkflowPolicyDescriptor, WorkflowSuccess,
    /// };
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Notify;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Notify {
    ///     fn on_workflow_success(
    ///         &self,
    ///         _got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .policy(WorkflowPolicyDescriptor::new("notify", || Notify).on_workflow_success())
    ///     .build()?;
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn policy<P: Send + Sync + 'static>(
        mut self,
        policy: WorkflowPolicyDescriptor<P, W, M>,
    ) -> Self {
        self.declaration.policies.push(Box::new(policy));
        self
    }

    /// Declares an input adapter of the workflow.
    ///
    /// # Panics
    ///
    /// Panics if the workflow already has an input adapter with the same name, since adapter
    /// names are unique within a workflow.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::{InputAdapterDescriptor, WorkflowDescriptor};
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .input_adapter(InputAdapterDescriptor::new("pricing", step_name!("charge"), |_, _| Ok(None)))
    ///     .build()?;
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    #[expect(
        clippy::panic,
        reason = "two adapters with one name are a mistake in a declaration, before any journey"
    )]
    pub fn input_adapter(mut self, adapter: InputAdapterDescriptor<W>) -> Self {
        if self
            .declaration
            .adapters
            .iter()
            .any(|known| known.is_named(adapter.name))
        {
            panic!(
                "the workflow already has an input adapter named \"{}\"",
                adapter.name
            );
        }
        self.declaration.adapters.push(adapter);
        self
    }

    /// Declares a service provider of the workflow, after those already declared.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::service::{Service, ServiceProviderDescriptor, provide};
    /// use itinera::step::{Outcome, Resolved, Step, StepDescriptor, StepFactory, StepNeeds, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// trait StockStore: Send + Sync {
    ///     fn reserve(&self) -> Result<(), Error>;
    /// }
    ///
    /// struct Warehouse;
    ///
    /// impl StockStore for Warehouse {
    ///     fn reserve(&self) -> Result<(), Error> {
    ///         Ok(())
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
    ///         self.stock.reserve()?;
    ///         Ok(Outcome::success())
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
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn provider<S: ProviderScope>(
        mut self,
        provider: ServiceProviderDescriptor<W, S, M>,
    ) -> Self {
        self.declaration.providers.push(Box::new(provider));
        self
    }

    /// Lists a reporter, which each instance makes for its journey with `Default`. Reporters
    /// receive events in the order they are listed, before those added as an instance is created
    /// with [`InstanceBuilder::reporter`](crate::instance::InstanceBuilder::reporter).
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::report::Reporter;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// #[derive(Default)]
    /// struct Audit;
    ///
    /// impl Reporter for Audit {
    ///     fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .reporter::<Audit>()
    ///     .build()?;
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn reporter<R: Reporter + Default>(mut self) -> Self {
        self.declaration.reporters.push(made::<R, M>);
        self
    }

    /// Gives the workflow its own way to produce journey IDs, from the workflow's own value and
    /// the initial data. Without one, a journey ID is a UUID v4. An error makes creating the
    /// instance fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::journey::DataBag;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// fn order_number(_: &Orders, data: &DataBag) -> Result<String, Error> {
    ///     let number = data
    ///         .get("number")
    ///         .and_then(|value| value.downcast_ref::<i64>())
    ///         .ok_or_else(|| Error::msg("the order has no number"))?;
    ///     Ok(format!("order-{number}"))
    /// }
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .id_generator(order_number)
    ///     .build()?;
    /// let instance = orders.instance(Orders).data("number", 7_i64).create()?;
    /// # use itinera::instance::WorkflowInstance;
    /// assert_eq!(instance.journey_id().to_string(), "order-7");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn id_generator(
        mut self,
        generate: impl Fn(&W, &DataBag) -> Result<String, Error> + Send + Sync + 'static,
    ) -> Self {
        self.declaration.id_generator = Some(Box::new(generate));
        self
    }

    /// Checks the declaration and builds the workflow descriptor, or returns every violation
    /// found: steps with the same name, a hook defined twice for one step or for the workflow,
    /// input adapters attached to a step the workflow does not have, or to a step another adapter
    /// is attached to, and services wrongly requested or provided.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::{Violation, ViolationKind, WorkflowDescriptor};
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// assert_eq!(orders.name().to_string(), "orders");
    ///
    /// let refused = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build();
    /// let kinds: Vec<ViolationKind> = refused.unwrap_err().iter().map(Violation::kind).collect();
    /// assert_eq!(kinds, [ViolationKind::DuplicateStepName]);
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn build(self) -> Result<WorkflowDescriptor<W, M>, Violations> {
        violation::check(&self.declaration)?;
        Ok(WorkflowDescriptor {
            declaration: Arc::new(self.declaration),
        })
    }
}

fn made<R: Reporter + Default, M: Mode>() -> M::Reporter {
    report::held::<M>(R::default())
}

#[cfg(test)]
mod tests {
    use crate::workflow::fixtures::{adapter, orders};

    #[test]
    #[should_panic(expected = "the workflow already has an input adapter named \"pricing\"")]
    fn two_input_adapters_with_one_name_cannot_be_declared() {
        let _ = orders()
            .input_adapter(adapter("pricing", "charge"))
            .input_adapter(adapter("pricing", "ship"));
    }
}
