//! Workflow types that know their own descriptor.

use super::WorkflowDescriptor;
use crate::mode::Mode;

/// A workflow type that knows its own descriptor, declared once and shared by every instance.
///
/// `#[workflow]`, the attribute of the `macros` feature, implements it on the workflow's struct,
/// holding the descriptor in a hidden static that is built the first time it is asked for. The
/// descriptor lists the workflow's steps, with `Orders::descriptor().listing()`.
///
/// # Examples
///
/// ```
/// use std::sync::LazyLock;
///
/// use itinera::executor::LocalExecutor;
/// use itinera::journey::JourneyStatus;
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
/// assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
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
}
