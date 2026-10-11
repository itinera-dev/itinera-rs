//! Declaring a workflow, one part at a time, and building its descriptor.

use std::sync::Arc;

use super::{
    Declaration, InputAdapterDescriptor, Violations, WorkflowDescriptor, WorkflowName, violation,
};
use crate::error::Error;
use crate::journey::{DataBag, JourneyId};
use crate::mode::{Mode, Synchronous};
use crate::policy::WorkflowPolicyDescriptor;
use crate::report::{Reporter, WorkflowReporter};
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

    /// Lists a reporter, which each instance makes for its journey. Reporters receive events in
    /// the order they are listed.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::event::Event;
    /// use itinera::journey::{DataBag, JourneyId};
    /// use itinera::report::{Reporter, WorkflowReporter};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// struct Audit;
    ///
    /// impl Reporter for Audit {
    ///     fn report(&mut self, _event: &Event) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// impl WorkflowReporter<Orders> for Audit {
    ///     fn init(_: &Orders, _: &JourneyId, _: &DataBag) -> Result<Self, Error> {
    ///         Ok(Audit)
    ///     }
    /// }
    ///
    /// let orders = WorkflowDescriptor::builder("orders").reporter::<Audit>().build()?;
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn reporter<R: WorkflowReporter<W>>(mut self) -> Self {
        self.declaration
            .reporters
            .push(Box::new(make_reporter::<W, R, M>));
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
    /// found: steps with the same name, a hook defined twice for one step or for the workflow, and
    /// input adapters attached to a step the workflow does not have, or to a step another adapter
    /// is attached to.
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
        violation::check(
            &self.declaration.steps,
            &self.declaration.policies,
            &self.declaration.adapters,
        )?;
        Ok(WorkflowDescriptor {
            declaration: Arc::new(self.declaration),
        })
    }
}

fn make_reporter<W, R: WorkflowReporter<W>, M: Mode>(
    workflow: &W,
    journey_id: &JourneyId,
    data: &DataBag,
) -> Result<M::Reporter, Error> {
    R::init(workflow, journey_id, data).map(held::<R, M>)
}

/// A reporter as a workflow in mode `M` holds it.
fn held<R: Reporter, M: Mode>(reporter: R) -> M::Reporter {
    let reporter: Box<dyn Reporter> = Box::new(reporter);
    M::Reporter::from(reporter)
}
