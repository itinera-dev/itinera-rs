//! Workflows: their declaration, the workflow descriptor, shared by every instance.

use std::num::NonZeroUsize;
use std::sync::Arc;

use uuid::Builder;

use crate::error::Error;
use crate::instance::{InstanceBuilder, Resolution};
use crate::journey::{DataBag, JourneyId};
use crate::mode::{Mode, Synchronous};
use crate::policy::{PolicyName, StepPolicyDescriptor, WorkflowPolicyDescriptor};
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
    steps: Vec<StepDescriptor<M>>,
    policies: Vec<WorkflowPolicyDescriptor>,
    adapters: Vec<InputAdapter<W>>,
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
    /// use itinera::policy::{StepHook, StepPolicyDescriptor};
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::{InputAdapter, WorkflowDescriptor};
    ///
    /// struct Orders;
    ///
    /// let audit = StepPolicyDescriptor::new("audit", StepHook::OnStepSuccess);
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())).policy(audit))
    ///     .step(StepDescriptor::new(step_name!("ship"), || Ok(Outcome::success())))
    ///     .input_adapter(InputAdapter::new("pricing", step_name!("charge"), |_, _, _| Ok(None)))
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

    pub(crate) fn steps(&self) -> &[StepDescriptor<M>] {
        &self.declaration.steps
    }

    /// The value of one input of a step: from the step's input adapter, if it has one that
    /// supplies it, and otherwise from the data bag.
    pub(crate) fn data_for_step(
        &self,
        workflow: &W,
        data: &DataBag,
        step: StepName,
        key: &str,
    ) -> Resolution {
        let adapted = self
            .declaration
            .adapters
            .iter()
            .find(|adapter| adapter.is_attached_to(step))
            .map(|adapter| adapter.adapt(workflow, step, key));
        match adapted {
            Some(Resolution::Absent) | None => from_data_bag(data, key),
            Some(answer) => answer,
        }
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
    pub fn step(mut self, step: StepDescriptor<M>) -> Self {
        self.declaration.steps.push(step);
        self
    }

    /// Attaches a workflow policy, after those already attached.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{WorkflowHook, WorkflowPolicyDescriptor};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let notify = WorkflowPolicyDescriptor::new("notify", WorkflowHook::OnWorkflowSuccess);
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .policy(notify)
    ///     .build()?;
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    pub fn policy(mut self, policy: WorkflowPolicyDescriptor) -> Self {
        self.declaration.policies.push(policy);
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
    /// use itinera::workflow::{InputAdapter, WorkflowDescriptor};
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::<Orders>::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .input_adapter(InputAdapter::new("pricing", step_name!("charge"), |_, _, _| Ok(None)))
    ///     .build()?;
    /// # Ok::<(), itinera::workflow::Violations>(())
    /// ```
    #[expect(
        clippy::panic,
        reason = "two adapters with one name are a mistake in a declaration, before any journey"
    )]
    pub fn input_adapter(mut self, adapter: InputAdapter<W>) -> Self {
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

/// An input adapter of a workflow: its name, the steps it is attached to, at least one, and the
/// function that adapts their inputs.
///
/// It is declared on the workflow with [`WorkflowBuilder::input_adapter`]. When one of its steps is
/// built, it is called for each of the step's inputs, in the order the step declares them, with
/// the workflow's own value, the step's name and the input's key:
///
/// - a value is the input's value, which must have the type the step declares;
/// - `None` means the adapter does not supply this input, which is read from the data bag;
/// - an error aborts the journey with `step could not be built`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{StepName, step_name};
/// use itinera::value::AnyValue;
/// use itinera::workflow::InputAdapter;
///
/// struct Orders {
///     price: i64,
/// }
///
/// fn pricing(orders: &Orders, _: StepName, key: &str) -> Result<Option<AnyValue>, Error> {
///     Ok((key == "amount").then(|| AnyValue::new(orders.price)))
/// }
///
/// let pricing = InputAdapter::new("pricing", step_name!("charge"), pricing)
///     .step(step_name!("refund"));
/// assert_eq!(pricing.name().to_string(), "pricing");
/// ```
#[derive(derive_more::Debug)]
pub struct InputAdapter<W> {
    name: AdapterName,
    steps: Vec<StepName>,
    #[debug(skip)]
    adapt: Box<Adapt<W>>,
}

type Adapt<W> = dyn Fn(&W, StepName, &str) -> Result<Option<AnyValue>, Error> + Send + Sync;

impl<W> InputAdapter<W> {
    /// Declares an input adapter with its name, a step it is attached to, and its function.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::step_name;
    /// use itinera::workflow::InputAdapter;
    ///
    /// struct Orders;
    ///
    /// let nothing = InputAdapter::new("nothing", step_name!("charge"), |_: &Orders, _, _| Ok(None));
    /// ```
    pub fn new(
        name: impl Into<AdapterName>,
        step: StepName,
        adapt: impl Fn(&W, StepName, &str) -> Result<Option<AnyValue>, Error> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            steps: vec![step],
            adapt: Box::new(adapt),
        }
    }

    /// Attaches the adapter to another step. A step it is already attached to is not added again.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::step_name;
    /// use itinera::workflow::InputAdapter;
    ///
    /// struct Orders;
    ///
    /// let nothing = InputAdapter::new("nothing", step_name!("charge"), |_: &Orders, _, _| Ok(None))
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
    /// use itinera::workflow::InputAdapter;
    ///
    /// struct Orders;
    ///
    /// let nothing = InputAdapter::new("nothing", step_name!("charge"), |_: &Orders, _, _| Ok(None));
    /// assert_eq!(nothing.name().to_string(), "nothing");
    /// ```
    pub fn name(&self) -> AdapterName {
        self.name
    }

    pub(crate) fn steps(&self) -> &[StepName] {
        &self.steps
    }

    fn is_named(&self, name: AdapterName) -> bool {
        self.name == name
    }

    fn is_attached_to(&self, step: StepName) -> bool {
        self.steps.contains(&step)
    }

    /// What the adapter answers for one input of one of its steps.
    fn adapt(&self, workflow: &W, step: StepName, key: &str) -> Resolution {
        match (self.adapt)(workflow, step, key) {
            Ok(Some(value)) => Resolution::Supplied {
                adapter: self.name,
                value,
            },
            Ok(None) => Resolution::Absent,
            Err(error) => Resolution::AdapterFailed {
                adapter: self.name,
                error,
            },
        }
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
    step: &StepDescriptor<M>,
    position: NonZeroUsize,
    adapters: &[InputAdapter<W>],
) -> ListedStep {
    let name = step.name();
    ListedStep {
        step: name,
        position,
        policies: step
            .policies()
            .iter()
            .map(StepPolicyDescriptor::name)
            .collect(),
        adapter: adapters
            .iter()
            .find(|adapter| adapter.is_attached_to(name))
            .map(InputAdapter::<W>::name),
    }
}

fn from_data_bag(data: &DataBag, key: &str) -> Resolution {
    data.get(key)
        .cloned()
        .map_or(Resolution::Absent, Resolution::InDataBag)
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
    use crate::policy::StepHook;
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
                .policy(StepPolicyDescriptor::new("audit", StepHook::OnStepSuccess))
                .policy(StepPolicyDescriptor::new("alarm", StepHook::OnStepFailure)),
            )
            .step(StepDescriptor::new(StepName::new("ship"), move || {
                shipped.fetch_add(1, Ordering::SeqCst);
                Ok(Outcome::success())
            }))
            .input_adapter(InputAdapter::new(
                "pricing",
                StepName::new("charge"),
                |_, _, _| Ok(None),
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
