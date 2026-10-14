//! The macro that makes the local executor of an instance's mode.

use super::LocalExecutor;
#[cfg(feature = "async")]
use super::asynchronous::AsyncLocalExecutor;
use crate::instance::WorkflowInstance;
#[cfg(feature = "async")]
use crate::mode::Asynchronous;
use crate::mode::{Mode, Synchronous};
#[cfg(feature = "async")]
use crate::report::AsyncDispatcherFactory;
use crate::report::DispatcherFactory;

mod sealed;

/// Makes the local executor of the instance's mode: a [`LocalExecutor`] for a `Synchronous`
/// workflow, an `AsyncLocalExecutor` for an `Asynchronous` one. Written
/// `local_executor_for!(instance)`, for an executor that uses the [`DefaultDispatcherFactory`], or
/// `local_executor_for!(instance, dispatcher_factory = factory)`, for one that creates the
/// dispatcher of every journey with `factory`.
///
/// It is given the instance itself, not a reference to it (`*instance` names the instance a
/// reference points to), and only borrows it, so the instance is then given to the executor's
/// `run`. The expression is evaluated, so it names an instance already made rather than making
/// one. The caller still writes what that `run` needs: an asynchronous executor's result is
/// awaited.
///
/// [`DefaultDispatcherFactory`]: crate::report::DefaultDispatcherFactory
///
/// # Examples
///
/// ```
/// use itinera::executor::local_executor_for;
/// use itinera::journey::StatusKind;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .build()?;
/// let instance = orders.instance(Orders).create()?;
/// let result = local_executor_for!(instance).run(instance)?;
/// assert_eq!(result.status.kind(), StatusKind::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A value that is not a dispatcher factory of the instance's mode does not compile:
///
/// ```compile_fail,E0277
/// use itinera::executor::local_executor_for;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .build()?;
/// let instance = orders.instance(Orders).create()?;
/// let executor = local_executor_for!(instance, dispatcher_factory = 42);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __local_executor_for {
    ($instance:expr $(,)?) => {
        $crate::executor::__local_executor_of(&$instance, $crate::report::DefaultDispatcherFactory)
    };
    ($instance:expr, dispatcher_factory = $factory:expr $(,)?) => {
        $crate::executor::__local_executor_of(&$instance, $factory)
    };
}

#[doc(inline)]
pub use crate::__local_executor_for as local_executor_for;

/// A dispatcher factory that the local executor of `M`'s workflows takes: a
/// [`DispatcherFactory`] for `Synchronous`, an [`AsyncDispatcherFactory`] for `Asynchronous`.
///
/// [`local_executor_for!`] requires it of the factory it is given. No other type can implement
/// it.
///
/// [`AsyncDispatcherFactory`]: crate::report::AsyncDispatcherFactory
///
/// # Examples
///
/// ```
/// use itinera::executor::{LocalDispatcherFactory, local_executor_for};
/// use itinera::instance::WorkflowInstance;
/// use itinera::journey::StatusKind;
/// use itinera::report::DefaultDispatcherFactory;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// fn executor_for<I: WorkflowInstance, F: LocalDispatcherFactory<I::Mode>>(
///     instance: &I,
///     factory: F,
/// ) -> F::LocalExecutor {
///     local_executor_for!(*instance, dispatcher_factory = factory)
/// }
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .build()?;
/// let instance = orders.instance(Orders).create()?;
/// let result = executor_for(&instance, DefaultDispatcherFactory).run(instance)?;
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

/// The local executor of the instance's mode, with the factory: what [`local_executor_for!`]
/// expands to.
#[doc(hidden)]
pub fn __local_executor_of<I: WorkflowInstance, F: LocalDispatcherFactory<I::Mode>>(
    _instance: &I,
    factory: F,
) -> F::LocalExecutor {
    factory.__local_executor()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::Refusal;
    use crate::executor::fixtures::{FailingFactory, Shop, recorded};
    use crate::journey::JourneyStatus;

    #[test]
    fn a_synchronous_instance_gets_the_local_executor() {
        let instance = recorded(Shop::new()).create().unwrap();

        let mut executor: LocalExecutor = local_executor_for!(instance);

        let result = executor.run(instance).unwrap();
        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    }

    #[test]
    fn the_executor_creates_each_journeys_dispatcher_with_the_factory_it_is_given() {
        let instance = recorded(Shop::new()).create().unwrap();

        let refusal =
            local_executor_for!(instance, dispatcher_factory = FailingFactory { on: None })
                .run(instance)
                .unwrap_err();

        assert!(matches!(refusal, Refusal::DispatcherFactory(_)));
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_instance_gets_the_asynchronous_local_executor() {
        let (instance, _) = crate::executor::fixtures::mixed_instance(Shop::new());

        let mut executor: AsyncLocalExecutor = local_executor_for!(instance);

        let result = futures::executor::block_on(executor.run(instance)).unwrap();
        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    }
}
