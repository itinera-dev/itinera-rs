//! Executors, which run workflow instances, and the refusals they give when a journey cannot
//! start.

use crate::engine::{self, Failures, Inline, WorkflowPolicies};
use crate::error::Error;
use crate::instance::WorkflowInstance;
use crate::journey::JourneyResult;
use crate::mode::Synchronous;
use crate::policy::{BuiltWorkflowPolicy, PolicyName, WorkflowPolicyEntry};
use crate::report::{DefaultDispatcherFactory, Dispatcher, DispatcherFactory};

#[cfg(feature = "async")]
mod asynchronous;
#[cfg(test)]
pub(crate) mod fixtures;

#[cfg(feature = "async")]
pub use asynchronous::AsyncLocalExecutor;

/// Runs synchronous workflows in process, one journey at a time, with no persistence: a journey
/// lives only as long as the call to [`run`](LocalExecutor::run).
///
/// It is given a [`DispatcherFactory`], or uses [`DefaultDispatcherFactory`]. Nothing of a
/// journey remains in it once `run` returns, so it can run many journeys one after another.
///
/// # Examples
///
/// ```
/// use itinera::executor::LocalExecutor;
/// use itinera::journey::StatusKind;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .build()?;
/// let mut executor = LocalExecutor::new();
/// let result = executor.run(orders.instance(Orders).create()?)?;
/// assert_eq!(result.status.kind(), StatusKind::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct LocalExecutor<F = DefaultDispatcherFactory> {
    factory: F,
}

impl LocalExecutor {
    /// Makes an executor that uses the [`DefaultDispatcherFactory`].
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::LocalExecutor;
    ///
    /// let executor = LocalExecutor::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

impl<F: DispatcherFactory> LocalExecutor<F> {
    /// Makes an executor that creates the dispatcher of every journey with this factory.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::LocalExecutor;
    /// use itinera::report::DefaultDispatcherFactory;
    ///
    /// let executor = LocalExecutor::with_dispatcher_factory(DefaultDispatcherFactory);
    /// ```
    pub fn with_dispatcher_factory(factory: F) -> Self {
        Self { factory }
    }

    /// Runs one journey of the instance and returns its result.
    ///
    /// Before the journey starts, it builds one instance of each of the workflow's policies, then
    /// creates the journey's dispatcher and adds the instance's reporters to it. Whatever happens
    /// inside the journey, failures and aborts included, is in the result. A panic is not caught:
    /// it reaches the caller, and the journey stops where it was.
    ///
    /// `run` takes the instance, so an instance runs one journey and cannot run again. It borrows
    /// the executor mutably, so an executor runs one journey at a time.
    ///
    /// # Errors
    ///
    /// A [`Refusal`], with no journey and no event, when a workflow policy fails while it is
    /// built, the dispatcher factory fails, or the dispatcher fails while the reporters are added.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::LocalExecutor;
    /// use itinera::journey::JourneyStatus;
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// let orders = WorkflowDescriptor::builder("orders")
    ///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
    ///     .build()?;
    /// let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    /// let result = LocalExecutor::new().run(instance)?;
    /// let JourneyStatus::Succeeded { data, .. } = result.status else {
    ///     panic!("the journey did not succeed");
    /// };
    /// assert!(data.get("amount").is_some());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn run<I: WorkflowInstance<Mode = Synchronous>>(
        &mut self,
        mut instance: I,
    ) -> Result<JourneyResult, Refusal> {
        let policies = build_policies(&instance)?;
        let mut dispatcher = self.factory.create().map_err(Refusal::DispatcherFactory)?;
        let failures = Failures::default();
        instance
            .take_reporters()
            .into_iter()
            .map(|reporter| failures.guard(reporter))
            .try_for_each(|reporter| dispatcher.add(reporter))
            .map_err(Refusal::Dispatcher)?;
        Ok(engine::finish(engine::run(
            instance,
            policies,
            Inline::from(dispatcher),
            failures,
            std::time::SystemTime::now,
        )))
    }
}

/// Why `run` did not start a journey: custom code called before the journey starts failed. There
/// is no journey and no event.
///
/// It names what refused the journey, and carries its error.
///
/// # Examples
///
/// ```
/// use itinera::executor::Refusal;
///
/// fn blames_the_dispatcher(refusal: &Refusal) -> bool {
///     matches!(refusal, Refusal::DispatcherFactory(_) | Refusal::Dispatcher(_))
/// }
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    /// A workflow policy failed while it was built for the journey.
    #[error("the workflow policy {policy} could not be built: {error}")]
    #[non_exhaustive]
    WorkflowPolicy {
        /// The policy's name.
        policy: PolicyName,
        /// The error it failed with.
        error: Error,
    },
    /// The dispatcher factory failed to create the journey's dispatcher.
    #[error("the dispatcher factory failed: {0}")]
    DispatcherFactory(Error),
    /// The dispatcher failed while the journey's reporters were added.
    #[error("the dispatcher failed while the reporters were added: {0}")]
    Dispatcher(Error),
}

/// Builds one instance of each of the instance's workflow policies, for its journey, in the order
/// they were attached.
pub(crate) fn build_policies<I: WorkflowInstance>(
    instance: &I,
) -> Result<WorkflowPolicies<I::Workflow, I::Mode>, Refusal> {
    instance
        .descriptor()
        .policies()
        .iter()
        .map(Box::as_ref)
        .map(build_policy)
        .collect()
}

fn build_policy<W, M>(
    policy: &dyn WorkflowPolicyEntry<W, M>,
) -> Result<Box<dyn BuiltWorkflowPolicy<W, M>>, Refusal> {
    match policy.build() {
        Ok(built) => Ok(built),
        Err(error) => Err(Refusal::WorkflowPolicy {
            policy: policy.name(),
            error,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::executor::fixtures::{FailingFactory, Shop, entries, run, run_with, shop_builder};
    use crate::journey::JourneyStatus;
    use crate::policy::WorkflowPolicyDescriptor;
    use crate::policy::tests::Quiet;
    use crate::report::DefaultDispatcher;
    use crate::step::{Outcome, StepDescriptor, StepName};

    #[test]
    fn a_journey_emits_its_events_in_order_to_every_reporter_and_succeeds_with_its_data() {
        let (result, log) = run(Shop::new());

        assert_eq!(result.journey_id.to_string(), "order-7");
        let JourneyStatus::Succeeded { data } = result.status else {
            panic!("the journey did not succeed: {:?}", result.status);
        };
        assert_eq!(data.keys().collect::<Vec<_>>(), ["amount"]);
        assert_eq!(
            entries(&log),
            [
                "audit journey_started",
                "fragile journey_started",
                "metrics journey_started",
                "audit attempt_started",
                "fragile attempt_started",
                "metrics attempt_started",
                "audit step_succeeded",
                "fragile step_succeeded",
                "metrics step_succeeded",
                "audit journey_succeeded",
                "fragile journey_succeeded",
                "metrics journey_succeeded",
            ]
        );
    }

    #[test]
    fn a_dispatcher_factory_that_fails_refuses_the_journey_before_any_event() {
        let (result, log) = run_with(FailingFactory { on: None });

        let Err(Refusal::DispatcherFactory(error)) = result else {
            panic!("the journey was not refused by the factory");
        };
        assert_eq!(error.to_string(), "no dispatcher today");
        assert!(entries(&log).is_empty());
    }

    #[test]
    fn a_dispatcher_that_fails_while_reporters_are_added_refuses_the_journey_before_any_event() {
        let (result, log) = run_with(FailingFactory { on: Some(None) });

        let Err(refusal) = result else {
            panic!("the journey was not refused");
        };
        assert!(matches!(refusal, Refusal::Dispatcher(_)));
        assert_eq!(
            refusal.to_string(),
            "the dispatcher failed while the reporters were added: the dispatcher refuses reporters"
        );
        assert!(entries(&log).is_empty());
    }

    struct Counting {
        created: usize,
    }

    impl DispatcherFactory for Counting {
        type Dispatcher = DefaultDispatcher;

        fn create(&mut self) -> Result<DefaultDispatcher, Error> {
            self.created += 1;
            Ok(DefaultDispatcher::new())
        }
    }

    #[test]
    fn an_executor_runs_journeys_one_after_another_each_with_a_new_dispatcher() {
        let workflow = shop_builder()
            .step(StepDescriptor::new(StepName::new("charge"), || {
                Ok(Outcome::success())
            }))
            .build()
            .unwrap();
        let mut executor = LocalExecutor::with_dispatcher_factory(Counting { created: 0 });

        let first = executor
            .run(workflow.instance(Shop::new()).create().unwrap())
            .unwrap();
        let second = executor
            .run(workflow.instance(Shop::new()).create().unwrap())
            .unwrap();

        assert_ne!(first.journey_id, second.journey_id);
        assert_eq!(executor.factory.created, 2);
    }

    #[test]
    fn a_workflow_policy_that_cannot_be_built_refuses_the_journey_before_its_dispatcher() {
        let broken = WorkflowPolicyDescriptor::fallible("notify", || {
            Err::<Quiet, _>(Error::msg("no mail server"))
        })
        .on_workflow_success();
        let workflow = shop_builder().policy(broken).build().unwrap();
        let mut executor = LocalExecutor::with_dispatcher_factory(Counting { created: 0 });

        let refused = executor.run(workflow.instance(Shop::new()).create().unwrap());

        let Err(refusal) = refused else {
            panic!("the journey was not refused");
        };
        assert!(matches!(
            &refusal,
            Refusal::WorkflowPolicy { policy, .. } if *policy == PolicyName::from("notify")
        ));
        assert_eq!(
            refusal.to_string(),
            "the workflow policy notify could not be built: no mail server"
        );
        assert_eq!(executor.factory.created, 0);
    }

    #[test]
    fn each_journey_gets_new_workflow_policy_instances() {
        let built = Arc::new(Mutex::new(0));
        let counting = Arc::clone(&built);
        let notify = WorkflowPolicyDescriptor::new("notify", move || {
            *counting.lock().unwrap() += 1;
            Quiet
        })
        .on_workflow_success();
        let workflow = shop_builder().policy(notify).build().unwrap();
        let mut executor = LocalExecutor::new();

        for _ in 0..2 {
            executor
                .run(workflow.instance(Shop::new()).create().unwrap())
                .unwrap();
        }

        assert_eq!(*built.lock().unwrap(), 2);
    }
}
