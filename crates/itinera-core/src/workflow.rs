//! Workflows: their declaration, the workflow descriptor, shared by every instance.

use std::sync::Arc;

use uuid::Builder;

use crate::error::Error;
use crate::instance::InstanceBuilder;
use crate::journey::{DataBag, JourneyId};
use crate::mode::{Mode, Synchronous};
use crate::report::{Reporter, WorkflowReporter};
use crate::step::StepName;

#[cfg(feature = "async")]
mod asynchronous;

/// A workflow's name, fixed when the program is compiled.
///
/// # Examples
///
/// ```
/// use itinera::workflow::WorkflowName;
///
/// let orders = WorkflowName::from("orders");
/// let text: &str = orders.as_ref();
/// assert_eq!(text, "orders");
/// assert_eq!(orders.to_string(), "orders");
/// ```
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
)]
#[as_ref(forward)]
pub struct WorkflowName(&'static str);

/// An input adapter's name, fixed when the program is compiled, unique among its workflow's
/// adapters.
///
/// # Examples
///
/// ```
/// use itinera::workflow::AdapterName;
///
/// let pricing = AdapterName::from("pricing");
/// let text: &str = pricing.as_ref();
/// assert_eq!(text, "pricing");
/// assert_eq!(pricing.to_string(), "pricing");
/// ```
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    derive_more::AsRef,
)]
#[as_ref(forward)]
pub struct AdapterName(&'static str);

/// A workflow's declaration: its name, its step, its reporters and how it produces journey IDs.
///
/// It is fixed once built, shared by every instance of the workflow, and cheap to clone. Only
/// [`WorkflowBuilder::build`] makes one. Its mode `M` is [`Synchronous`] unless the workflow has
/// an asynchronous part.
///
/// # Examples
///
/// ```
/// use std::sync::LazyLock;
///
/// use itinera::step::step_name;
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// static ORDERS: LazyLock<WorkflowDescriptor<Orders>> = LazyLock::new(|| {
///     WorkflowDescriptor::builder("orders")
///         .step(step_name!("charge"), || {})
///         .build()
/// });
///
/// assert_eq!(ORDERS.name().to_string(), "orders");
/// ```
#[derive(derive_more::Debug)]
pub struct WorkflowDescriptor<W, M: Mode = Synchronous> {
    declaration: Arc<Declaration<W, M>>,
}

#[derive(derive_more::Debug)]
struct Declaration<W, M: Mode> {
    name: WorkflowName,
    step: Option<StandInStep>,
    #[debug("{}", reporters.len())]
    reporters: Vec<MakeReporter<W, M>>,
    #[debug("{}", id_generator.is_some())]
    id_generator: Option<GenerateId<W>>,
}

type MakeReporter<W, M> =
    Box<dyn Fn(&W, &JourneyId, &DataBag) -> Result<<M as Mode>::Reporter, Error> + Send + Sync>;

type GenerateId<W> = Box<dyn Fn(&W, &DataBag) -> Result<String, Error> + Send + Sync>;

/// A stand-in for a step, which can only succeed, until steps can be declared in full.
#[derive(derive_more::Debug)]
pub(crate) struct StandInStep {
    name: StepName,
    #[debug(skip)]
    run: Box<dyn Fn() + Send + Sync>,
}

impl StandInStep {
    pub(crate) fn name(&self) -> StepName {
        self.name
    }

    pub(crate) fn run(&self) {
        (self.run)();
    }
}

impl<W: Send + Sync + 'static> WorkflowDescriptor<W> {
    /// Starts declaring a workflow with this name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders").build();
    /// assert_eq!(orders.name().to_string(), "orders");
    /// ```
    pub fn builder(name: impl Into<WorkflowName>) -> WorkflowBuilder<W> {
        WorkflowBuilder {
            declaration: Declaration {
                name: name.into(),
                step: None,
                reporters: Vec::new(),
                id_generator: None,
            },
        }
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
    /// let orders = WorkflowDescriptor::builder("orders").build();
    /// let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    /// # drop(instance);
    /// # Ok::<(), itinera::instance::InstanceError>(())
    /// ```
    pub fn instance(&self, workflow: W) -> InstanceBuilder<W, M> {
        InstanceBuilder::new(self.clone(), workflow)
    }

    pub(crate) fn step(&self) -> Option<&StandInStep> {
        self.declaration.step.as_ref()
    }

    /// The journey ID for a new instance: the workflow's generator's, or a UUID v4.
    pub(crate) fn journey_id(&self, workflow: &W, data: &DataBag) -> Result<JourneyId, Error> {
        match &self.declaration.id_generator {
            Some(generate) => generate(workflow, data).map(JourneyId::from),
            None => random_id(),
        }
    }

    /// The workflow's reporters, made for one journey, in the order they were listed.
    pub(crate) fn reporters(
        &self,
        workflow: &W,
        journey_id: &JourneyId,
        data: &DataBag,
    ) -> Result<Vec<M::Reporter>, Error> {
        self.declaration
            .reporters
            .iter()
            .map(|make| make(workflow, journey_id, data))
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
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders").build();
    /// assert_eq!(orders.name().to_string(), "orders");
    /// ```
    pub fn name(&self) -> WorkflowName {
        self.declaration.name
    }
}

impl<W, M: Mode> Clone for WorkflowDescriptor<W, M> {
    fn clone(&self) -> Self {
        Self {
            declaration: Arc::clone(&self.declaration),
        }
    }
}

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
///     .build();
/// let instance = orders.instance(Orders { prefix: "order" }).create()?;
/// # drop(instance);
/// # Ok::<(), itinera::instance::InstanceError>(())
/// ```
#[derive(derive_more::Debug)]
pub struct WorkflowBuilder<W, M: Mode = Synchronous> {
    declaration: Declaration<W, M>,
}

impl<W: Send + Sync + 'static, M: Mode> WorkflowBuilder<W, M> {
    /// Gives the workflow its step, replacing any step given before.
    ///
    /// The step is a stand-in that can only succeed, until steps can be declared in full: the
    /// journey runs it once, and it succeeds.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::step_name;
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .step(step_name!("charge"), || {})
    ///     .build();
    /// # drop(orders);
    /// ```
    pub fn step(mut self, name: StepName, run: impl Fn() + Send + Sync + 'static) -> Self {
        self.declaration.step = Some(StandInStep {
            name,
            run: Box::new(run),
        });
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
    /// let orders = WorkflowDescriptor::builder("orders").reporter::<Audit>().build();
    /// # drop(orders);
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
    ///     .build();
    /// let instance = orders.instance(Orders).data("number", 7_i64).create()?;
    /// # use itinera::instance::WorkflowInstance;
    /// assert_eq!(instance.journey_id().to_string(), "order-7");
    /// # Ok::<(), itinera::instance::InstanceError>(())
    /// ```
    pub fn id_generator(
        mut self,
        generate: impl Fn(&W, &DataBag) -> Result<String, Error> + Send + Sync + 'static,
    ) -> Self {
        self.declaration.id_generator = Some(Box::new(generate));
        self
    }

    /// Builds the workflow descriptor.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders").build();
    /// assert_eq!(orders.clone().name().to_string(), "orders");
    /// ```
    pub fn build(self) -> WorkflowDescriptor<W, M> {
        WorkflowDescriptor {
            declaration: Arc::new(self.declaration),
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
