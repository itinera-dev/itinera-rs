//! Executors, which run workflow instances, and the refusals they give when a journey cannot
//! start.

use crate::engine::{self, Failures, Inline, WorkflowPolicies};
use crate::instance::WorkflowInstance;
use crate::journey::JourneyResult;
use crate::mode::Synchronous;
use crate::policy::{BuiltWorkflowPolicy, WorkflowPolicyEntry};
use crate::report::{DefaultDispatcherFactory, Dispatcher, DispatcherFactory, GlobalMetadata};

#[cfg(feature = "async")]
mod asynchronous;
#[cfg(test)]
pub(crate) mod fixtures;
mod refusal;

#[cfg(feature = "async")]
pub use asynchronous::AsyncLocalExecutor;
pub use refusal::Refusal;

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
    /// Before the journey starts, it builds one instance of each of the workflow's policies,
    /// creates the journey's dispatcher and adds the instance's reporters to it, then sets the
    /// journey's global metadata: the journey ID, under [`GlobalMetadata::JOURNEY_ID`], then the
    /// instance's initial entries. Whatever happens inside the journey, failures and aborts
    /// included, is in the result. A panic is not caught: it reaches the caller, and the journey
    /// stops where it was.
    ///
    /// `run` takes the instance, so an instance runs one journey and cannot run again. It borrows
    /// the executor mutably, so an executor runs one journey at a time.
    ///
    /// # Errors
    ///
    /// A [`Refusal`], with no journey and no event, when the initial global metadata sets
    /// `journey_id`, a workflow policy fails while it is built, the dispatcher factory fails, the
    /// dispatcher fails while the reporters are added, or a reporter or the dispatcher fails
    /// while the global metadata is set.
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
        let metadata = initial_global_metadata(&mut instance)?;
        let policies = build_policies(&instance)?;
        let mut dispatcher = self.factory.create().map_err(Refusal::DispatcherFactory)?;
        let failures = Failures::default();
        instance
            .take_reporters()
            .into_iter()
            .map(|reporter| failures.guard(reporter))
            .try_for_each(|reporter| dispatcher.add(reporter))
            .map_err(Refusal::Dispatcher)?;
        let mut delivery = Inline::from(dispatcher);
        let journey_id = instance.journey_id();
        engine::finish(engine::introduce(
            &mut delivery,
            &failures,
            journey_id,
            metadata,
        ))
        .map_err(Refusal::GlobalMetadata)?;
        Ok(engine::finish(engine::run(
            instance,
            policies,
            delivery,
            failures,
            std::time::SystemTime::now,
        )))
    }
}

/// Takes the instance's initial global metadata, unless it sets the journey ID's own key.
pub(crate) fn initial_global_metadata<I: WorkflowInstance>(
    instance: &mut I,
) -> Result<GlobalMetadata, Refusal> {
    let metadata = instance.take_global_metadata();
    if metadata.sets_journey_id() {
        return Err(Refusal::ReservedMetadataKey);
    }
    Ok(metadata)
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
mod tests;
