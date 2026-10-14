//! The dispatcher factories that the local executor of each mode takes.

use super::sealed;
use crate::executor::LocalExecutor;
#[cfg(feature = "async")]
use crate::executor::asynchronous::AsyncLocalExecutor;
#[cfg(feature = "async")]
use crate::mode::Asynchronous;
use crate::mode::{Mode, Synchronous};
#[cfg(feature = "async")]
use crate::report::AsyncDispatcherFactory;
use crate::report::DispatcherFactory;

/// A dispatcher factory that the local executor of `M`'s workflows takes: a
/// [`DispatcherFactory`] for `Synchronous`, an [`AsyncDispatcherFactory`] for `Asynchronous`.
///
/// [`local_executor_for!`] requires it of the factory it is given. No other type can implement
/// it.
///
/// [`AsyncDispatcherFactory`]: crate::report::AsyncDispatcherFactory
/// [`local_executor_for!`]: crate::executor::local_executor_for
///
/// # Examples
///
/// ```
/// use itinera::executor::{LocalDispatcherFactory, local_executor_for};
/// use itinera::journey::StatusKind;
/// use itinera::mode::HasMode;
/// use itinera::report::DefaultDispatcherFactory;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// fn executor_for<T: HasMode, F: LocalDispatcherFactory<T::Mode>>(factory: F) -> F::LocalExecutor {
///     local_executor_for!(T, dispatcher_factory = factory)
/// }
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .build()?;
/// let instance = orders.instance(Orders).create()?;
/// let result =
///     executor_for::<WorkflowDescriptor<Orders>, _>(DefaultDispatcherFactory).run(instance)?;
/// assert_eq!(result.status.kind(), StatusKind::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a dispatcher factory that the local executor of a `{M}` workflow \
               takes",
    label = "not a dispatcher factory of this workflow's executor",
    note = "a `Synchronous` workflow's executor takes a `DispatcherFactory`, and an \
            `Asynchronous` workflow's executor takes an `AsyncDispatcherFactory`"
)]
pub trait LocalDispatcherFactory<M: Mode>: sealed::Sealed<M> {
    /// The local executor of `M`'s workflows that creates the dispatcher of every journey with
    /// this factory.
    type LocalExecutor;

    #[doc(hidden)]
    fn __local_executor(self) -> Self::LocalExecutor;
}

impl<F: DispatcherFactory> LocalDispatcherFactory<Synchronous> for F {
    type LocalExecutor = LocalExecutor<F>;

    fn __local_executor(self) -> LocalExecutor<F> {
        LocalExecutor::with_dispatcher_factory(self)
    }
}

#[cfg(feature = "async")]
impl<F: AsyncDispatcherFactory> LocalDispatcherFactory<Asynchronous> for F {
    type LocalExecutor = AsyncLocalExecutor<F>;

    fn __local_executor(self) -> AsyncLocalExecutor<F> {
        AsyncLocalExecutor::with_dispatcher_factory(self)
    }
}
