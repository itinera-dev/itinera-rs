//! Workflows: their declaration, the workflow descriptor, shared by every instance.

use std::num::NonZeroUsize;
use std::sync::Arc;

use uuid::Builder;

use crate::error::Error;
use crate::instance::InstanceBuilder;
use crate::journey::{DataBag, JourneyId};
use crate::mode::{Mode, Synchronous};
use crate::policy::{
    HookNeeds, InputAdapter, Needs, PolicyName, Requested, WorkflowPolicyDescriptor,
    WorkflowPolicyEntry,
};
use crate::report::{Reporter, WorkflowReporter};
use crate::step::{StepDescriptor, StepName};
use crate::value::AnyValue;

#[cfg(feature = "async")]
mod asynchronous;
mod violation;

pub use violation::{Violation, ViolationKind, Violations};

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

/// A workflow's declaration: its name, its step descriptors in order, its workflow policies, its
/// input adapters, its reporters and how it produces journey IDs.
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

#[derive(derive_more::Debug)]
struct Declaration<W, M: Mode> {
    name: WorkflowName,
    steps: Vec<StepDescriptor<W, M>>,
    policies: Vec<Box<dyn WorkflowPolicyEntry<W, M>>>,
    adapters: Vec<InputAdapterDescriptor<W>>,
    #[debug("{}", reporters.len())]
    reporters: Vec<MakeReporter<W, M>>,
    #[debug("{}", id_generator.is_some())]
    id_generator: Option<GenerateId<W>>,
}

type MakeReporter<W, M> =
    Box<dyn Fn(&W, &JourneyId, &DataBag) -> Result<<M as Mode>::Reporter, Error> + Send + Sync>;

type GenerateId<W> = Box<dyn Fn(&W, &DataBag) -> Result<String, Error> + Send + Sync>;

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

    /// Lists the workflow's steps in order, without running anything: each step's name, its
    /// position from 1, the names of its policies in the order they were attached, and the name of
    /// its input adapter, if it has one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     OnStepSuccess, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess,
    /// };
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::{InputAdapterDescriptor, WorkflowDescriptor};
    ///
    /// struct Audit;
    ///
    /// impl<W: Send + Sync + 'static> OnStepSuccess<W> for Audit {
    ///     fn on_step_success(
    ///         &self,
    ///         _got: Requested<'_, W, StepSuccess>,
    ///     ) -> Result<Option<OnSuccess>, Error> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let audit = StepPolicyDescriptor::new("audit", || Audit).on_step_success();
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .step(
    ///         StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())).policy(audit),
    ///     )
    ///     .step(StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success())))
    ///     .input_adapter(InputAdapterDescriptor::new("pricing", step_name!("charge"), |_, _| Ok(None)))
    ///     .build()?;
    ///
    /// let listing = orders.listing();
    /// let charge = &listing[0];
    /// assert_eq!(charge.step, step_name!("charge"));
    /// assert_eq!(charge.position.get(), 1);
    /// assert_eq!(charge.policies.len(), 1);
    /// assert!(charge.adapter.is_some());
    /// assert!(listing[1].adapter.is_none());
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn listing(&self) -> Vec<ListedStep> {
        let adapters = &self.declaration.adapters;
        self.declaration
            .steps
            .iter()
            .zip(positions())
            .map(|(step, position)| listed(step, position, adapters))
            .collect()
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
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders").build()?;
    /// assert_eq!(orders.name().to_string(), "orders");
    /// # Ok::<(), itinera::workflow::Violations>(())
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
///     .build()?;
/// let instance = orders.instance(Orders { prefix: "order" }).create()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(derive_more::Debug)]
pub struct WorkflowBuilder<W, M: Mode = Synchronous> {
    declaration: Declaration<W, M>,
}

impl<W: Send + Sync + 'static, M: Mode> WorkflowBuilder<W, M> {
    fn named(name: WorkflowName) -> Self {
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

/// The description of an input adapter of workflows `W`: its name, the steps it is attached to,
/// at least one, what it needs, and its function. An input adapter is a hook of the workflow
/// itself, of the kind [`InputAdapter`].
///
/// It is declared on the workflow with [`WorkflowBuilder::input_adapter`]. When one of its steps is
/// built, it is called for each of the step's inputs, in the order the step declares them, with
/// the workflow's own value and what it [`Requested`]: the step's name, the input's key, the
/// journey ID and the data from the workflow it declared, read from the data bag, and, if it
/// declared it, read access to the data bag.
///
/// - A value is the input's value, which must have the type the step declares.
/// - `None` means the adapter does not supply this input, which is read from the data bag.
/// - An error aborts the journey with `step could not be built`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{InputAdapter, Requested};
/// use itinera::step::step_name;
/// use itinera::value::AnyValue;
/// use itinera::workflow::InputAdapterDescriptor;
///
/// struct Orders {
///     price: i64,
/// }
///
/// impl Orders {
///     fn pricing(&self, got: Requested<'_, Self, InputAdapter>) -> Result<Option<AnyValue>, Error> {
///         Ok((got.key() == "amount").then(|| AnyValue::new(self.price)))
///     }
/// }
///
/// let pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), Orders::pricing)
///     .step(step_name!("refund"));
/// assert_eq!(pricing.name().to_string(), "pricing");
/// ```
#[derive(derive_more::Debug)]
pub struct InputAdapterDescriptor<W> {
    name: AdapterName,
    steps: Vec<StepName>,
    needs: Needs,
    #[debug(skip)]
    adapt: Box<Adapt<W>>,
}

type Adapt<W> = dyn for<'a> Fn(&'a W, Requested<'a, W, InputAdapter>) -> Result<Option<AnyValue>, Error>
    + Send
    + Sync;

impl<W> InputAdapterDescriptor<W> {
    /// Describes an input adapter with its name, a step it is attached to, and its function. It
    /// needs nothing, until [`needing`](Self::needing) says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::step_name;
    /// use itinera::workflow::InputAdapterDescriptor;
    ///
    /// struct Orders;
    ///
    /// let nothing = InputAdapterDescriptor::new("nothing", step_name!("charge"), |_: &Orders, _| Ok(None));
    /// ```
    pub fn new(
        name: impl Into<AdapterName>,
        step: StepName,
        adapt: impl for<'a> Fn(&'a W, Requested<'a, W, InputAdapter>) -> Result<Option<AnyValue>, Error>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            steps: vec![step],
            needs: Needs::default(),
            adapt: Box::new(adapt),
        }
    }

    /// Declares what the adapter needs, resolved before each of its calls: data from the workflow,
    /// and read access to the data bag.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{HookNeeds, InputAdapter, Requested};
    /// use itinera::step::{Input, step_name};
    /// use itinera::value::AnyValue;
    /// use itinera::workflow::InputAdapterDescriptor;
    ///
    /// const PRICE: Input<i64> = Input::new("price");
    ///
    /// struct Orders;
    ///
    /// impl Orders {
    ///     fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Option<AnyValue>, Error> {
    ///         let price = got.from_workflow(&PRICE)?;
    ///         Ok((got.key() == "amount").then(|| AnyValue::new(price)))
    ///     }
    /// }
    ///
    /// let pricing = InputAdapterDescriptor::new("pricing", step_name!("charge"), Orders::pricing)
    ///     .needing(HookNeeds::new().from_workflow(&PRICE));
    /// ```
    pub fn needing(mut self, needs: HookNeeds<InputAdapter>) -> Self {
        self.needs = needs.into();
        self
    }

    /// Attaches the adapter to another step. A step it is already attached to is not added again.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::step_name;
    /// use itinera::workflow::InputAdapterDescriptor;
    ///
    /// struct Orders;
    ///
    /// let nothing = InputAdapterDescriptor::new("nothing", step_name!("charge"), |_: &Orders, _| Ok(None))
    ///     .step(step_name!("refund"));
    /// ```
    pub fn step(mut self, step: StepName) -> Self {
        if !self.steps.contains(&step) {
            self.steps.push(step);
        }
        self
    }

    /// The adapter's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::step_name;
    /// use itinera::workflow::InputAdapterDescriptor;
    ///
    /// struct Orders;
    ///
    /// let nothing = InputAdapterDescriptor::new("nothing", step_name!("charge"), |_: &Orders, _| Ok(None));
    /// assert_eq!(nothing.name().to_string(), "nothing");
    /// ```
    pub fn name(&self) -> AdapterName {
        self.name
    }

    pub(crate) fn steps(&self) -> &[StepName] {
        &self.steps
    }

    pub(crate) fn needs(&self) -> &Needs {
        &self.needs
    }

    fn is_named(&self, name: AdapterName) -> bool {
        self.name == name
    }

    fn is_attached_to(&self, step: StepName) -> bool {
        self.steps.contains(&step)
    }

    /// Calls the adapter for one input of one of its steps.
    pub(crate) fn adapt<'a>(
        &self,
        workflow: &'a W,
        got: Requested<'a, W, InputAdapter>,
    ) -> Result<Option<AnyValue>, Error> {
        (self.adapt)(workflow, got)
    }
}

/// One step of a workflow's listing.
///
/// # Examples
///
/// ```
/// use itinera::workflow::ListedStep;
///
/// fn describe(listed: &ListedStep) -> String {
///     format!("{}. {}", listed.position, listed.step)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListedStep {
    /// The step's name.
    pub step: StepName,
    /// Where the step runs among the workflow's steps, from 1.
    pub position: NonZeroUsize,
    /// The step policies attached to it, in the order they were attached.
    pub policies: Vec<PolicyName>,
    /// The input adapter attached to it, if any.
    pub adapter: Option<AdapterName>,
}

fn listed<M, W>(
    step: &StepDescriptor<W, M>,
    position: NonZeroUsize,
    adapters: &[InputAdapterDescriptor<W>],
) -> ListedStep {
    let name = step.name();
    ListedStep {
        step: name,
        position,
        policies: step.policies().iter().map(|policy| policy.name()).collect(),
        adapter: attached_to(adapters, name).map(InputAdapterDescriptor::<W>::name),
    }
}

/// The input adapter attached to the step, among the workflow's adapters.
fn attached_to<W>(
    adapters: &[InputAdapterDescriptor<W>],
    step: StepName,
) -> Option<&InputAdapterDescriptor<W>> {
    adapters.iter().find(|adapter| adapter.is_attached_to(step))
}

/// The positions of steps: 1, 2, 3 and so on.
fn positions() -> impl Iterator<Item = NonZeroUsize> {
    std::iter::successors(Some(NonZeroUsize::MIN), next_position)
}

fn next_position(position: &NonZeroUsize) -> Option<NonZeroUsize> {
    position.checked_add(1)
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::policy::StepPolicyDescriptor;
    use crate::policy::tests::Quiet;
    use crate::step::Outcome;

    struct Orders;

    #[test]
    fn a_workflow_is_listed_in_order_with_its_policies_and_adapters_without_running_anything() {
        let ran = Arc::new(AtomicUsize::new(0));
        let charged = Arc::clone(&ran);
        let shipped = Arc::clone(&ran);
        let orders = WorkflowDescriptor::<Orders>::builder("orders")
            .step(
                StepDescriptor::new(StepName::new("charge"), move || {
                    charged.fetch_add(1, Ordering::SeqCst);
                    Ok(Outcome::success())
                })
                .policy(StepPolicyDescriptor::new("audit", || Quiet).on_step_success())
                .policy(StepPolicyDescriptor::new("alarm", || Quiet).on_step_failure()),
            )
            .step(StepDescriptor::new(StepName::new("ship"), move || {
                shipped.fetch_add(1, Ordering::SeqCst);
                Ok(Outcome::success())
            }))
            .input_adapter(InputAdapterDescriptor::new(
                "pricing",
                StepName::new("charge"),
                |_, _| Ok(None),
            ))
            .build()
            .unwrap();

        assert_eq!(
            orders.listing(),
            [
                ListedStep {
                    step: StepName::new("charge"),
                    position: NonZeroUsize::new(1).unwrap(),
                    policies: vec![PolicyName::from("audit"), PolicyName::from("alarm")],
                    adapter: Some(AdapterName::from("pricing")),
                },
                ListedStep {
                    step: StepName::new("ship"),
                    position: NonZeroUsize::new(2).unwrap(),
                    policies: Vec::new(),
                    adapter: None,
                },
            ]
        );
        assert_eq!(ran.load(Ordering::SeqCst), 0);
    }
}
