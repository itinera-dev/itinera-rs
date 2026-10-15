//! Workflow types that know their own descriptor.

use super::WorkflowDescriptor;
use crate::error::Error;
use crate::mode::Mode;
use crate::value::{AnyValue, Value};

/// A workflow type that knows its own descriptor, declared once and shared by every instance.
///
/// `#[workflow]`, the attribute of the `macros` feature, implements it on the workflow's struct,
/// holding the descriptor in a hidden static that is built the first time it is asked for. The
/// descriptor lists the workflow's steps, with `Orders::descriptor().listing()`. A workflow's
/// input adapters may answer with [`Workflow::supply`] or [`Workflow::from_data_bag`].
///
/// # Examples
///
/// ```
/// use std::sync::LazyLock;
///
/// use itinera::executor::LocalExecutor;
/// use itinera::mode::Synchronous;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::{Workflow, WorkflowDescriptor};
///
/// struct Orders;
///
/// impl Workflow for Orders {
///     type Mode = Synchronous;
///
///     fn descriptor() -> &'static WorkflowDescriptor<Self> {
///         static DESCRIPTOR: LazyLock<WorkflowDescriptor<Orders>> = LazyLock::new(|| {
///             WorkflowDescriptor::builder("orders")
///                 .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///                 .build()
///                 .expect("the orders workflow is well formed")
///         });
///         &DESCRIPTOR
///     }
/// }
///
/// assert_eq!(Orders::descriptor().listing().len(), 1);
/// let instance = Orders::descriptor().instance(Orders).create()?;
/// let result = LocalExecutor::new().run(instance)?;
/// assert!(result.status.is_succeeded());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a workflow that knows its own descriptor",
    label = "not a described workflow",
    note = "write `#[workflow(...)]` on its struct, with a `#[with_step(...)]` line per step"
)]
pub trait Workflow: Send + Sync + Sized + 'static {
    /// The workflow's mode: `Synchronous`, or `Asynchronous` for a workflow whose steps,
    /// policies and reporters are asynchronous.
    type Mode: Mode;

    /// The workflow's descriptor, the same every time it is asked for.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::{Workflow, WorkflowName};
    ///
    /// fn named<W: Workflow>() -> WorkflowName {
    ///     W::descriptor().name()
    /// }
    /// ```
    fn descriptor() -> &'static WorkflowDescriptor<Self, Self::Mode>;

    /// An input adapter supplies this value for the input it was asked for.
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that an input adapter can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::value::AnyValue;
    /// use itinera::workflow::Workflow;
    ///
    /// fn discounted<W: Workflow>(price: i64, discount: i64) -> Result<Option<AnyValue>> {
    ///     W::supply(price - discount)
    /// }
    /// ```
    fn supply<T: Value>(value: T) -> Result<Option<AnyValue>, Error> {
        Ok(Some(AnyValue::new(value)))
    }

    /// An input adapter supplies nothing, so the input is read from the data bag.
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that an input adapter can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::value::AnyValue;
    /// use itinera::workflow::Workflow;
    ///
    /// fn as_given<W: Workflow>() -> Result<Option<AnyValue>> {
    ///     W::from_data_bag()
    /// }
    /// ```
    fn from_data_bag() -> Result<Option<AnyValue>, Error> {
        Ok(None)
    }
}
