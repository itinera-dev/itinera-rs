//! Input adapters: hooks of the workflow itself, which supply the inputs of the steps they are
//! attached to.

use super::{AdapterName, Supplied};
use crate::error::Error;
use crate::policy::{HookNeeds, Needs, Requested};
use crate::step::StepName;

mod kind;

pub use kind::InputAdapter;

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
/// - [`Supplied::Value`] is the input's value, which must have the type the step declares.
/// - [`Supplied::FromDataBag`] means the adapter does not supply this input, which is read from
///   the data bag.
/// - An error aborts the journey with `step could not be built`.
///
/// [`WorkflowBuilder::input_adapter`]: super::WorkflowBuilder::input_adapter
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::policy::Requested;
/// use itinera::step::step_name;
/// use itinera::workflow::{InputAdapter, InputAdapterDescriptor, Supplied};
///
/// struct Orders {
///     price: i64,
/// }
///
/// impl Orders {
///     fn pricing(&self, got: Requested<'_, Self, InputAdapter>) -> Result<Supplied> {
///         Ok(if got.key() == "amount" {
///             Supplied::value(self.price)
///         } else {
///             Supplied::FromDataBag
///         })
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

type Adapt<W> =
    dyn for<'a> Fn(&'a W, Requested<'a, W, InputAdapter>) -> Result<Supplied, Error> + Send + Sync;

impl<W> InputAdapterDescriptor<W> {
    /// Describes an input adapter with its name, a step it is attached to, and its function. It
    /// needs nothing, until [`needing`](Self::needing) says otherwise. A rescue step is attached
    /// to by its name, such as `charge_rescue`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::step_name;
    /// use itinera::workflow::{InputAdapterDescriptor, Supplied};
    ///
    /// struct Orders;
    ///
    /// let nothing = InputAdapterDescriptor::new("nothing", step_name!("charge"), |_: &Orders, _| Ok(Supplied::FromDataBag));
    /// ```
    pub fn new(
        name: impl Into<AdapterName>,
        step: StepName,
        adapt: impl for<'a> Fn(&'a W, Requested<'a, W, InputAdapter>) -> Result<Supplied, Error>
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
    /// use itinera::error::Result;
    /// use itinera::policy::{HookNeeds, Requested};
    /// use itinera::step::{Input, step_name};
    /// use itinera::workflow::{InputAdapter, InputAdapterDescriptor, Supplied};
    ///
    /// const PRICE: Input<i64> = Input::new("price");
    ///
    /// struct Orders;
    ///
    /// impl Orders {
    ///     fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Supplied> {
    ///         let price = got.from_workflow(&PRICE)?;
    ///         Ok(if got.key() == "amount" {
    ///             Supplied::value(price)
    ///         } else {
    ///             Supplied::FromDataBag
    ///         })
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

    /// Attaches the adapter to another step, or to a rescue step by its name, such as
    /// `charge_rescue`. Attached to a step twice, the adapter adapts it twice, so the workflow is
    /// refused with [`Violation::StepAdaptedTwice`](crate::workflow::Violation::StepAdaptedTwice).
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::step_name;
    /// use itinera::workflow::{InputAdapterDescriptor, Supplied};
    ///
    /// struct Orders;
    ///
    /// let nothing = InputAdapterDescriptor::new("nothing", step_name!("charge"), |_: &Orders, _| Ok(Supplied::FromDataBag))
    ///     .step(step_name!("refund"));
    /// ```
    pub fn step(mut self, step: StepName) -> Self {
        self.steps.push(step);
        self
    }

    /// The adapter's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::step_name;
    /// use itinera::workflow::{InputAdapterDescriptor, Supplied};
    ///
    /// struct Orders;
    ///
    /// let nothing = InputAdapterDescriptor::new("nothing", step_name!("charge"), |_: &Orders, _| Ok(Supplied::FromDataBag));
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

    pub(super) fn is_attached_to(&self, step: StepName) -> bool {
        self.steps.contains(&step)
    }

    /// Calls the adapter for one input of one of its steps.
    pub(crate) fn adapt<'a>(
        &self,
        workflow: &'a W,
        got: Requested<'a, W, InputAdapter>,
    ) -> Result<Supplied, Error> {
        (self.adapt)(workflow, got)
    }
}
