//! The types that know the mode of their workflow.

use super::Mode;
use crate::instance::{Instance, WorkflowInstance};
use crate::workflow::{Workflow, WorkflowDescriptor};

mod sealed;

/// A type that knows the mode of its workflow: a workflow that knows its own descriptor (the
/// [`Workflow`] that `#[workflow]` implements), a [`WorkflowDescriptor`] or an [`Instance`]. No
/// other type can implement it.
///
/// [`local_executor_for!`](crate::executor::local_executor_for) reads the mode through it.
///
/// # Examples
///
/// ```
/// use itinera::mode::{HasMode, Synchronous};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// fn synchronous<T: HasMode<Mode = Synchronous>>() {}
///
/// synchronous::<WorkflowDescriptor<Orders>>();
/// ```
pub trait HasMode: sealed::Sealed {
    /// The mode of the workflow.
    type Mode: Mode;
}

impl<W: Workflow> HasMode for W {
    type Mode = W::Mode;
}

impl<W, M: Mode> HasMode for WorkflowDescriptor<W, M> {
    type Mode = M;
}

impl<W, M: Mode> HasMode for Instance<W, M> {
    type Mode = M;
}

/// A value that knows the mode of its workflow, which `local_executor_for!(from = value)` is
/// given: any [`WorkflowInstance`], hand-written ones included, or a [`WorkflowDescriptor`].
#[doc(hidden)]
pub trait __ModeOfValue {
    type Mode: Mode;
}

impl<I: WorkflowInstance> __ModeOfValue for I {
    type Mode = I::Mode;
}

impl<W, M: Mode> __ModeOfValue for WorkflowDescriptor<W, M> {
    type Mode = M;
}
