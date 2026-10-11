//! Input adapters: hooks of the workflow itself, which supply the inputs of the steps they are
//! attached to.

use super::AdapterName;
use crate::error::Error;
use crate::policy::{HookNeeds, InputAdapter, Needs, Requested};
use crate::step::StepName;
use crate::value::AnyValue;

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
/// [`WorkflowBuilder::input_adapter`]: super::WorkflowBuilder::input_adapter
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
    pub(super) name: AdapterName,
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

    pub(super) fn is_named(&self, name: AdapterName) -> bool {
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

/// The input adapter attached to the step, among the workflow's adapters.
pub(super) fn attached_to<W>(
    adapters: &[InputAdapterDescriptor<W>],
    step: StepName,
) -> Option<&InputAdapterDescriptor<W>> {
    adapters.iter().find(|adapter| adapter.is_attached_to(step))
}
