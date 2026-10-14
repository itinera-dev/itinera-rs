//! The macro that makes the local executor of a workflow's mode.

use crate::mode::{__ModeOfValue, HasMode};

mod dispatcher_factory;

pub use dispatcher_factory::LocalDispatcherFactory;

/// Makes the local executor of a workflow's mode: a [`LocalExecutor`] for a `Synchronous`
/// workflow, an `AsyncLocalExecutor` for an `Asynchronous` one.
///
/// It is given a type that knows the mode, [`HasMode`]: `local_executor_for!(Orders)` for a
/// workflow that `#[workflow]` declares, `local_executor_for!(WorkflowDescriptor<Orders>)` or
/// `local_executor_for!(Instance<Orders>)`. So the executor can be made before any instance, and
/// run every journey after it. Given `from = value`, it reads the mode from the type of a
/// descriptor or of any instance, a hand-written one included, which it only borrows:
/// `local_executor_for!(from = instance)` leaves the instance to be given to the executor's `run`.
/// The value is the descriptor or the instance itself, not a reference to it (`*descriptor` names
/// the descriptor a reference points to).
///
/// Either form makes an executor that uses the [`DefaultDispatcherFactory`], or, followed by
/// `dispatcher_factory = factory`, one that creates the dispatcher of every journey with
/// `factory`. The caller still writes what the executor's `run` needs: an asynchronous executor's
/// result is awaited.
///
/// [`DefaultDispatcherFactory`]: crate::report::DefaultDispatcherFactory
/// [`LocalExecutor`]: super::LocalExecutor
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
    ($has_mode:ty $(,)?) => {
        $crate::executor::__local_executor_of::<$has_mode, _>(
            $crate::report::DefaultDispatcherFactory,
        )
    };
    ($has_mode:ty, dispatcher_factory = $factory:expr $(,)?) => {
        $crate::executor::__local_executor_of::<$has_mode, _>($factory)
    };
}

#[doc(inline)]
pub use crate::__local_executor_for as local_executor_for;

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
pub fn __local_executor_from<T: __ModeOfValue, F: LocalDispatcherFactory<T::Mode>>(
    _value: &T,
    factory: F,
) -> F::LocalExecutor {
    factory.__local_executor()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::executor::fixtures::{FailingFactory, HandWritten, Shop, recorded};
    use crate::executor::local_executor_for;
    use crate::executor::{LocalExecutor, Refusal};
    use crate::instance::{Instance, WorkflowInstance};
    use crate::journey::JourneyStatus;
    use crate::workflow::WorkflowDescriptor;

    type Pick = fn(&Instance<Shop>) -> LocalExecutor;

    type PickWith = fn(&Instance<Shop>, FailingFactory) -> LocalExecutor<FailingFactory>;

    fn shop_instance() -> Instance<Shop> {
        recorded(Shop::new()).create().unwrap()
    }

    fn by_its_descriptors_type(_: &Instance<Shop>) -> LocalExecutor {
        local_executor_for!(WorkflowDescriptor<Shop>)
    }

    fn by_its_instances_type(_: &Instance<Shop>) -> LocalExecutor {
        local_executor_for!(Instance<Shop>)
    }

    fn from_its_instance(instance: &Instance<Shop>) -> LocalExecutor {
        local_executor_for!(from = *instance)
    }

    fn from_its_descriptor(instance: &Instance<Shop>) -> LocalExecutor {
        local_executor_for!(from = *instance.descriptor())
    }

    fn from_a_hand_written_instance(_: &Instance<Shop>) -> LocalExecutor {
        local_executor_for!(
            from = HandWritten {
                instance: shop_instance()
            }
        )
    }

    fn by_its_type_with(
        _: &Instance<Shop>,
        factory: FailingFactory,
    ) -> LocalExecutor<FailingFactory> {
        local_executor_for!(Instance<Shop>, dispatcher_factory = factory)
    }

    fn from_its_instance_with(
        instance: &Instance<Shop>,
        factory: FailingFactory,
    ) -> LocalExecutor<FailingFactory> {
        local_executor_for!(from = *instance, dispatcher_factory = factory)
    }

    #[rstest]
    #[case::by_its_descriptors_type(by_its_descriptors_type)]
    #[case::by_its_instances_type(by_its_instances_type)]
    #[case::from_its_instance(from_its_instance)]
    #[case::from_its_descriptor(from_its_descriptor)]
    #[case::from_a_hand_written_instance(from_a_hand_written_instance)]
    fn a_synchronous_workflow_gets_the_local_executor(#[case] pick: Pick) {
        let instance = shop_instance();

        let mut executor = pick(&instance);

        let result = executor.run(instance).unwrap();
        assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
    }

    #[rstest]
    #[case::by_its_type(by_its_type_with)]
    #[case::from_a_value(from_its_instance_with)]
    fn the_executor_creates_each_journeys_dispatcher_with_the_factory_it_is_given(
        #[case] pick: PickWith,
    ) {
        let instance = shop_instance();

        let refusal = pick(&instance, FailingFactory { on: None })
            .run(instance)
            .unwrap_err();

        assert!(matches!(refusal, Refusal::DispatcherFactory(_)));
    }

    #[cfg(feature = "async")]
    mod asynchronous {
        use rstest::rstest;

        use crate::executor::AsyncLocalExecutor;
        use crate::executor::fixtures::{Shop, mixed_instance};
        use crate::executor::local_executor_for;
        use crate::instance::Instance;
        use crate::journey::JourneyStatus;
        use crate::mode::Asynchronous;
        use crate::workflow::WorkflowDescriptor;

        type Pick = fn(&Instance<Shop, Asynchronous>) -> AsyncLocalExecutor;

        fn by_its_descriptors_type(_: &Instance<Shop, Asynchronous>) -> AsyncLocalExecutor {
            local_executor_for!(WorkflowDescriptor<Shop, Asynchronous>)
        }

        fn from_its_instance(instance: &Instance<Shop, Asynchronous>) -> AsyncLocalExecutor {
            local_executor_for!(from = *instance)
        }

        #[rstest]
        #[case::by_its_descriptors_type(by_its_descriptors_type)]
        #[case::from_its_instance(from_its_instance)]
        fn an_asynchronous_workflow_gets_the_asynchronous_local_executor(#[case] pick: Pick) {
            let (instance, _) = mixed_instance(Shop::new());

            let mut executor = pick(&instance);

            let result = futures::executor::block_on(executor.run(instance)).unwrap();
            assert!(matches!(result.status, JourneyStatus::Succeeded { .. }));
        }
    }
}
