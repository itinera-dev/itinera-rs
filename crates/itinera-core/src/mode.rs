//! Execution modes: whether a workflow's parts are all synchronous, or some are asynchronous.

use crate::report::Reporter;

#[cfg(feature = "async")]
mod asynchronous;
mod sealed;

#[cfg(feature = "async")]
pub use asynchronous::Asynchronous;

/// An execution mode, part of a workflow descriptor's type, which decides the executors that
/// accept it.
///
/// It is [`Synchronous`] unless one of the workflow's parts is asynchronous, which makes it
/// `Asynchronous`, with the `async` feature. No other type can be a mode.
///
/// # Examples
///
/// ```
/// use itinera::mode::Mode;
/// use itinera::workflow::WorkflowDescriptor;
///
/// fn name<W, M: Mode>(descriptor: &WorkflowDescriptor<W, M>) -> &str {
///     descriptor.name()
/// }
/// ```
pub trait Mode: sealed::Sealed + Send + Sync + 'static {
    /// How a workflow in this mode holds each of its reporters.
    type Reporter: From<Box<dyn Reporter>> + Send + 'static;
}

/// The mode of a workflow whose parts are all synchronous, which every executor accepts.
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
///     WorkflowDescriptor::builder("orders").build();
/// # drop(orders);
/// ```
#[derive(Clone, Copy, Debug)]
pub enum Synchronous {}

impl Mode for Synchronous {
    type Reporter = Box<dyn Reporter>;
}
