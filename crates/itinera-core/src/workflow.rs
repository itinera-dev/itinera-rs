//! Workflows: their declaration, the workflow descriptor, shared by every instance.

use std::sync::Arc;

use uuid::Builder;

use crate::error::Error;
use crate::instance::InstanceBuilder;
use crate::journey::{DataBag, JourneyId};
use crate::mode::{Mode, Synchronous};
use crate::policy::WorkflowPolicyEntry;
#[cfg(feature = "unstable")]
use crate::service::{self, ProviderEntry};
use crate::step::{StepDescriptor, StepName};

#[cfg(feature = "async")]
mod asynchronous;
mod builder;
mod declaration;
mod described;
#[cfg(test)]
pub(crate) mod fixtures;
mod input_adapter;
mod listing;
mod name;
mod violation;

use declaration::Declaration;
use input_adapter::attached_to;

pub use builder::WorkflowBuilder;
pub use described::Workflow;
pub use input_adapter::InputAdapterDescriptor;
#[cfg(feature = "macros")]
pub use itinera_macros::{role, workflow};
#[cfg(feature = "unstable")]
pub use listing::ListedService;
pub use listing::ListedStep;
pub use name::{AdapterName, WorkflowName};
pub use violation::{Violation, ViolationKind, Violations};

/// A workflow's declaration: its name, its step descriptors in order, its workflow policies, its
/// input adapters, with the `unstable` feature its service providers, its reporters and how it
/// produces journey IDs.
///
/// It is fixed once built, shared by every instance of the workflow, and cheap to clone. Only
/// [`WorkflowBuilder::build`] makes one, after checking the declaration. Its mode `M` is
/// [`Synchronous`], or `Asynchronous` for a workflow declared with `async_builder`.
///
/// # Examples
///
/// ```
/// use std::sync::LazyLock;
///
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// static ORDERS: LazyLock<WorkflowDescriptor<Orders>> = LazyLock::new(|| {
///     WorkflowDescriptor::builder("orders")
///         .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///         .build()
///         .expect("the orders workflow is well formed")
/// });
///
/// assert_eq!(ORDERS.name().to_string(), "orders");
/// ```
#[derive(derive_more::Debug)]
pub struct WorkflowDescriptor<W, M: Mode = Synchronous> {
    declaration: Arc<Declaration<W, M>>,
}

impl<W: Send + Sync + 'static> WorkflowDescriptor<W> {
    /// Starts declaring a synchronous workflow with this name, which only the synchronous
    /// executor runs. Its steps and reporters are synchronous.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders").build()?;
    /// assert_eq!(orders.name().to_string(), "orders");
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn builder(name: impl Into<WorkflowName>) -> WorkflowBuilder<W> {
        WorkflowBuilder::named(name.into())
    }
}

impl<W: Send + Sync + 'static, M: Mode> WorkflowDescriptor<W, M> {
    /// Starts creating an instance of the workflow, for one journey, holding the workflow's own
    /// value.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders").build()?;
    /// let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn instance(&self, workflow: W) -> InstanceBuilder<W, M> {
        InstanceBuilder::new(self.clone(), workflow)
    }

    pub(crate) fn steps(&self) -> &[StepDescriptor<W, M>] {
        &self.declaration.steps
    }

    pub(crate) fn policies(&self) -> &[Box<dyn WorkflowPolicyEntry<W, M>>] {
        &self.declaration.policies
    }

    /// The input adapter attached to the step, if it has one.
    pub(crate) fn adapter(&self, step: StepName) -> Option<&InputAdapterDescriptor<W>> {
        attached_to(&self.declaration.adapters, step)
    }

    /// The journey ID for a new instance: the workflow's generator's, or a UUID v4.
    pub(crate) fn journey_id(&self, workflow: &W, data: &DataBag) -> Result<JourneyId, Error> {
        match &self.declaration.id_generator {
            Some(generate) => generate(workflow, data).map(JourneyId::from),
            None => random_id(),
        }
    }

    /// The workflow's listed reporters, made for one journey, in the order they were listed.
    pub(crate) fn reporters(&self) -> Vec<M::Reporter> {
        self.declaration
            .reporters
            .iter()
            .map(|make| make())
            .collect()
    }
}

impl<W, M: Mode> WorkflowDescriptor<W, M> {
    /// The workflow's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders").build()?;
    /// assert_eq!(orders.name().to_string(), "orders");
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn name(&self) -> WorkflowName {
        self.declaration.name
    }

    /// The workflow's service providers, in the order they were declared.
    #[cfg(feature = "unstable")]
    pub(crate) fn providers(&self) -> &[Box<dyn ProviderEntry<W, M>>] {
        &self.declaration.providers
    }

    /// The service provider that serves the step for the service under `key`, if one does.
    #[cfg(feature = "unstable")]
    pub(crate) fn provider(&self, step: StepName, key: &str) -> Option<&dyn ProviderEntry<W, M>> {
        service::serving(
            &self.declaration.providers,
            self.declaration.served(step),
            key,
        )
    }
}

impl<W, M: Mode> Clone for WorkflowDescriptor<W, M> {
    fn clone(&self) -> Self {
        Self {
            declaration: Arc::clone(&self.declaration),
        }
    }
}

/// A UUID v4, from the system's random source, which may fail.
fn random_id() -> Result<JourneyId, Error> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes)?;
    let uuid = Builder::from_random_bytes(bytes).into_uuid();
    Ok(JourneyId::from(uuid.to_string()))
}
