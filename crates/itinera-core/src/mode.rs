//! Execution modes: whether a workflow is synchronous or asynchronous.

use crate::report::Reporter;

#[cfg(feature = "async")]
mod asynchronous;
pub(crate) mod sealed;

#[cfg(feature = "async")]
pub use asynchronous::Asynchronous;

/// An execution mode, part of a workflow descriptor's type, which decides the executor that
/// accepts it.
///
/// A workflow is declared in its mode from the start: [`Synchronous`], or `Asynchronous`, with
/// the `async` feature. No other type can be a mode.
///
/// # Examples
///
/// ```
/// use itinera::mode::Mode;
/// use itinera::workflow::{WorkflowDescriptor, WorkflowName};
///
/// fn name<W, M: Mode>(descriptor: &WorkflowDescriptor<W, M>) -> WorkflowName {
///     descriptor.name()
/// }
/// ```
pub trait Mode: sealed::Sealed + Send + Sync + 'static {
    /// How a workflow in this mode holds each of its reporters.
    type Reporter: From<Box<dyn Reporter>> + Send + 'static;
}

/// The mode of a workflow whose parts are all synchronous, which only the synchronous executor
/// accepts.
///
/// # Examples
///
/// ```
/// use itinera::mode::Synchronous;
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders: WorkflowDescriptor<Orders, Synchronous> =
///     WorkflowDescriptor::builder("orders").build()?;
/// # Ok::<(), itinera::workflow::Violations>(())
/// ```
#[derive(Clone, Copy, Debug)]
pub enum Synchronous {}

impl Mode for Synchronous {
    type Reporter = Box<dyn Reporter>;
}
