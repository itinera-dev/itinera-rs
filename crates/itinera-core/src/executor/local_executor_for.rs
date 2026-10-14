//! The macro that makes the local executor of a workflow's mode.

use super::LocalExecutor;
#[cfg(feature = "async")]
use super::asynchronous::AsyncLocalExecutor;
#[cfg(feature = "async")]
use crate::mode::Asynchronous;
use crate::mode::{HasMode, Mode, Synchronous};
#[cfg(feature = "async")]
use crate::report::AsyncDispatcherFactory;
use crate::report::DispatcherFactory;

mod sealed;

/// Makes the local executor of a workflow's mode: a [`LocalExecutor`] for a `Synchronous`
/// workflow, an `AsyncLocalExecutor` for an `Asynchronous` one.
///
/// It is given a type that knows the mode, [`HasMode`]: `local_executor_for!(Orders)` for a
/// workflow that `#[workflow]` declares, or `local_executor_for!(WorkflowDescriptor<Orders>)`.
/// So the executor can be made before any instance, and run every journey after it. Given
/// `from = value`, it reads the mode from the type of a descriptor or an instance instead, which it
/// only borrows: `local_executor_for!(from = instance)` leaves the instance to be given to the
/// executor's `run`. The value is the descriptor or the instance itself, not a reference to it
/// (`*descriptor` names the descriptor a reference points to).
///
/// Either form makes an executor that uses the [`DefaultDispatcherFactory`], or, followed by
/// `dispatcher_factory = factory`, one that creates the dispatcher of every journey with
/// `factory`. The caller still writes what the executor's `run` needs: an asynchronous executor's
/// result is awaited.
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
/// let mut executor = local_executor_for!(WorkflowDescriptor<Orders>);
/// for _ in 0..2 {
///     let result = executor.run(orders.instance(Orders).create()?)?;
///     assert_eq!(result.status.kind(), StatusKind::Succeeded);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// From an instance:
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
/// let result = local_executor_for!(from = instance).run(instance)?;
/// assert_eq!(result.status.kind(), StatusKind::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// A type that does not know a workflow's mode does not compile:
///
/// ```compile_fail,E0277
/// use itinera::executor::local_executor_for;
///
/// struct Orders;
///
/// let executor = local_executor_for!(Orders);
/// ```
///
/// Nor does a value that is not a dispatcher factory of the workflow's mode:
///
/// ```compile_fail,E0277
/// use itinera::executor::local_executor_for;
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let executor = local_executor_for!(WorkflowDescriptor<Orders>, dispatcher_factory = 42);
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __local_executor_for {
    (from = $value:expr $(,)?) => {
        $crate::executor::__local_executor_from(&$value, $crate::report::DefaultDispatcherFactory)
    };
    (from = $value:expr, dispatcher_factory = $factory:expr $(,)?) => {
        $crate::executor::__local_executor_from(&$value, $factory)
    };
    ($workflow:ty $(,)?) => {
        $crate::executor::__local_executor_of::<$workflow, _>(
            $crate::report::DefaultDispatcherFactory,
        )
    };
    ($workflow:ty, dispatcher_factory = $factory:expr $(,)?) => {
        $crate::executor::__local_executor_of::<$workflow, _>($factory)
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

/// The local executor of `T`'s mode, with the factory: what [`local_executor_for!`] expands to
/// when given a type.
#[doc(hidden)]
pub fn __local_executor_of<T: HasMode, F: LocalDispatcherFactory<T::Mode>>(
    factory: F,
) -> F::LocalExecutor {
    factory.__local_executor()
}

/// The local executor of the value's mode, with the factory: what [`local_executor_for!`] expands
/// to when given `from = value`.
#[doc(hidden)]
pub fn __local_executor_from<T: HasMode, F: LocalDispatcherFactory<T::Mode>>(
    _value: &T,
    factory: F,
) -> F::LocalExecutor {
    __local_executor_of::<T, F>(factory)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::Refusal;
    use crate::executor::fixtures::{FailingFactory, Shop, recorded};
    use crate::instance::Instance;
    use crate::journey::JourneyStatus;
    use crate::workflow::WorkflowDescriptor;

    #[test]
    fn a_synchronous_workflow_gets_the_local_executor() {
        let mut executor: LocalExecutor = local_executor_for!(WorkflowDescriptor<Shop>);

        let result = executor
            .run(recorded(Shop::new()).create().unwrap())
            .unwrap();
        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    }

    #[test]
    fn a_synchronous_instance_gets_the_local_executor() {
        let instance = recorded(Shop::new()).create().unwrap();

        let mut executor: LocalExecutor = local_executor_for!(from = instance);

        let result = executor.run(instance).unwrap();
        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    }

    #[test]
    fn the_executor_creates_each_journeys_dispatcher_with_the_factory_it_is_given() {
        let refusal = local_executor_for!(
            Instance<Shop>,
            dispatcher_factory = FailingFactory { on: None }
        )
        .run(recorded(Shop::new()).create().unwrap())
        .unwrap_err();

        assert!(matches!(refusal, Refusal::DispatcherFactory(_)));
    }

    #[test]
    fn the_executor_made_from_a_value_creates_each_journeys_dispatcher_with_the_factory_it_is_given()
     {
        let instance = recorded(Shop::new()).create().unwrap();

        let refusal = local_executor_for!(
            from = instance,
            dispatcher_factory = FailingFactory { on: None }
        )
        .run(instance)
        .unwrap_err();

        assert!(matches!(refusal, Refusal::DispatcherFactory(_)));
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_workflow_gets_the_asynchronous_local_executor() {
        let mut executor: AsyncLocalExecutor =
            local_executor_for!(WorkflowDescriptor<Shop, Asynchronous>);

        let (instance, _) = crate::executor::fixtures::mixed_instance(Shop::new());
        let result = futures::executor::block_on(executor.run(instance)).unwrap();
        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    }

    #[cfg(feature = "async")]
    #[test]
    fn an_asynchronous_instance_gets_the_asynchronous_local_executor() {
        let (instance, _) = crate::executor::fixtures::mixed_instance(Shop::new());

        let mut executor: AsyncLocalExecutor = local_executor_for!(from = instance);

        let result = futures::executor::block_on(executor.run(instance)).unwrap();
        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    }
}
