use super::started::initial_global_metadata;
use super::{Refusal, Started, build_policies};
use crate::engine::{self, Awaited, Failures};
use crate::instance::WorkflowInstance;
use crate::journey::JourneyResult;
#[cfg(any(test, feature = "testing"))]
use crate::journey::ScriptedRun;
use crate::mode::Asynchronous;
use crate::report::{AsyncDispatcher, AsyncDispatcherFactory, DefaultDispatcherFactory};
#[cfg(any(test, feature = "testing"))]
use crate::step::Script;

/// Runs asynchronous workflows in process, one journey at a time, with no persistence: everything
/// asynchronous is awaited, and synchronous reporters are called inline.
///
/// It is given an [`AsyncDispatcherFactory`], or uses [`DefaultDispatcherFactory`]. Nothing of a
/// journey remains in it once `run` returns, so it can run many journeys one after another. It
/// needs no particular async runtime: it never spawns, sleeps or sets timers.
///
/// Dropping the future that [`run`](AsyncLocalExecutor::run) returns abandons the journey where
/// it is.
///
/// # Examples
///
/// ```
/// use itinera::executor::AsyncLocalExecutor;
/// use itinera::journey::StatusKind;
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Orders;
///
/// async fn charge() -> Result<StatusKind, Box<dyn std::error::Error>> {
///     let orders = WorkflowDescriptor::async_builder("orders")
///         .step(StepDescriptor::new_async(step_name!("charge"), async || Ok(Outcome::success())))
///         .build()?;
///     let mut executor = AsyncLocalExecutor::new();
///     let result = executor.run(orders.instance(Orders).create()?).await?;
///     Ok(result.status.kind())
/// }
///
/// assert_eq!(futures::executor::block_on(charge())?, StatusKind::Succeeded);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Default)]
pub struct AsyncLocalExecutor<F = DefaultDispatcherFactory> {
    factory: F,
}

impl AsyncLocalExecutor {
    /// Makes an executor that uses the [`DefaultDispatcherFactory`].
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::AsyncLocalExecutor;
    ///
    /// let executor = AsyncLocalExecutor::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
}

impl<F: AsyncDispatcherFactory> AsyncLocalExecutor<F> {
    /// Makes an executor that creates the dispatcher of every journey with this factory.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::executor::AsyncLocalExecutor;
    /// use itinera::report::DefaultDispatcherFactory;
    ///
    /// let executor = AsyncLocalExecutor::with_dispatcher_factory(DefaultDispatcherFactory);
    /// ```
    pub fn with_dispatcher_factory(factory: F) -> Self {
        Self { factory }
    }

    /// Runs one journey of the instance and returns its result.
    ///
    /// Before the journey starts, it builds one instance of each of the workflow's policies,
    /// creates the journey's dispatcher and adds the instance's reporters to it, then sets the
    /// journey's global metadata: the journey ID, under
    /// [`GlobalMetadata::JOURNEY_ID`](crate::report::GlobalMetadata::JOURNEY_ID), then the
    /// instance's initial entries. Whatever happens inside the journey, failures and aborts
    /// included, is in the result. A panic is not caught: it reaches the caller, and the journey
    /// stops where it was.
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
    /// use itinera::executor::AsyncLocalExecutor;
    /// use itinera::journey::{JourneyResult, StatusKind};
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    /// use itinera::workflow::WorkflowDescriptor;
    ///
    /// struct Orders;
    ///
    /// async fn charge() -> Result<JourneyResult, Box<dyn std::error::Error>> {
    ///     let orders = WorkflowDescriptor::async_builder("orders")
    ///         .step(StepDescriptor::new_async(step_name!("charge"), async || Ok(Outcome::success())))
    ///         .build()?;
    ///     let instance = orders.instance(Orders).data("amount", 42_i64).create()?;
    ///     Ok(AsyncLocalExecutor::new().run(instance).await?)
    /// }
    ///
    /// let result = futures::executor::block_on(charge())?;
    /// assert_eq!(result.status.kind(), StatusKind::Succeeded);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub async fn run<I: WorkflowInstance<Mode = Asynchronous>>(
        &mut self,
        instance: I,
    ) -> Result<JourneyResult, Refusal> {
        let started = self.start(instance).await?;
        Ok(engine::run(started, std::time::SystemTime::now).await)
    }

    /// Runs one journey of the instance as [`run`](Self::run) does, with each attempt the script
    /// plays played instead of running its step, and gives back its result with what every
    /// attempt's step was built with.
    #[cfg(any(test, feature = "testing"))]
    pub(crate) async fn run_scripted<I: WorkflowInstance<Mode = Asynchronous>>(
        &mut self,
        instance: I,
        script: Script,
    ) -> Result<ScriptedRun, Refusal> {
        let started = self.start(instance).await?;
        Ok(engine::run_scripted(started, script).await)
    }

    /// Prepares the instance's journey: builds the workflow's policies, creates the dispatcher
    /// with the instance's reporters, and sets the initial global metadata.
    async fn start<I: WorkflowInstance<Mode = Asynchronous>>(
        &mut self,
        mut instance: I,
    ) -> Result<Started<I, Awaited<F::Dispatcher>>, Refusal> {
        let metadata = initial_global_metadata(&mut instance)?;
        let policies = build_policies(&instance)?;
        let mut dispatcher = self
            .factory
            .create()
            .await
            .map_err(Refusal::DispatcherFactory)?;
        let failures = Failures::default();
        for reporter in instance.take_reporters() {
            dispatcher
                .add(failures.guard_boxed(reporter))
                .await
                .map_err(Refusal::Dispatcher)?;
        }
        let mut delivery = Awaited::from(dispatcher);
        engine::introduce(&mut delivery, &failures, instance.journey_id(), metadata)
            .await
            .map_err(Refusal::GlobalMetadata)?;
        Ok(Started {
            instance,
            policies,
            delivery,
            failures,
        })
    }
}

#[cfg(test)]
mod tests;
