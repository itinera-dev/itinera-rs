//! The executors that run the scenario's journeys, one for each execution mode.

use itinera::executor::{AsyncLocalExecutor, LocalExecutor, Refusal};
use itinera::instance::Instance;
use itinera::journey::JourneyResult;
use itinera::mode::{Asynchronous, Mode, Synchronous};
use itinera::report::{AsyncDispatcherFactory, DispatcherFactory};

/// An executor of the mode's workflows.
pub(crate) trait Runs<M: Mode> {
    async fn run<W: Send + Sync + 'static>(
        &mut self,
        instance: Instance<W, M>,
    ) -> Result<JourneyResult, Refusal>;
}

impl<F: DispatcherFactory> Runs<Synchronous> for LocalExecutor<F> {
    async fn run<W: Send + Sync + 'static>(
        &mut self,
        instance: Instance<W, Synchronous>,
    ) -> Result<JourneyResult, Refusal> {
        LocalExecutor::run(self, instance)
    }
}

impl<F: AsyncDispatcherFactory> Runs<Asynchronous> for AsyncLocalExecutor<F> {
    async fn run<W: Send + Sync + 'static>(
        &mut self,
        instance: Instance<W, Asynchronous>,
    ) -> Result<JourneyResult, Refusal> {
        AsyncLocalExecutor::run(self, instance).await
    }
}
