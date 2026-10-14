//! The same parts in an asynchronous workflow, doing the same as their synchronous twins.

use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::mode::Asynchronous;
use itinera::policy::{
    AsyncOnStepAbnormalTermination, AsyncOnStepFailure, AsyncOnStepRetry, AsyncOnStepSuccess,
    AsyncOnWorkflowFailure, AsyncOnWorkflowSuccess, FailWorkflow, HookNeeds, OnSuccess,
    PolicyHookKind, Requested, StepAbnormalTermination, StepFailure, StepRetry, StepSuccess,
    WorkflowFailure, WorkflowSuccess,
};
use itinera::step::{AsyncStep, AsyncStepFactory, AsyncStepReporter, Outcome, Resolved, StepNeeds};

use super::{Scripted, Verdict, closed, consulting, decided, ended, on_success};
use crate::journey::{BAG_VALUE, Shop};
use crate::plan::{Closing, Ending};

/// One attempt of a scripted step in an asynchronous workflow.
pub(crate) struct AsyncScriptedStep<'a> {
    ending: Ending,
    contributor: Contributor<'a>,
    reporter: AsyncStepReporter<'a>,
}

impl AsyncStepFactory for Scripted {
    type Step<'a> = AsyncScriptedStep<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().contributor().reporter()
    }

    fn build<'a>(
        &'a self,
        got: &mut Resolved<'a, Asynchronous>,
    ) -> Result<AsyncScriptedStep<'a>, Error> {
        Ok(AsyncScriptedStep {
            ending: self.next_ending(),
            contributor: got.contributor()?,
            reporter: got.reporter()?,
        })
    }
}

impl AsyncStep for AsyncScriptedStep<'_> {
    async fn run(mut self) -> Result<Outcome, Error> {
        self.contributor.contribute("receipt", BAG_VALUE.to_owned());
        self.reporter.info("working").await?;
        ended(self.ending)
    }
}

impl AsyncOnStepSuccess<Shop> for Verdict {
    fn needs() -> HookNeeds<StepSuccess> {
        consulting()
    }

    async fn on_step_success(
        &self,
        mut got: Requested<'_, Shop, StepSuccess, Asynchronous>,
    ) -> Result<Option<OnSuccess>, Error> {
        consult(&mut got).await?;
        on_success(self.planned(got.attempt()).on_step_success)
    }
}

impl AsyncOnStepFailure<Shop> for Verdict {
    fn needs() -> HookNeeds<StepFailure> {
        consulting()
    }

    async fn on_step_failure(
        &self,
        mut got: Requested<'_, Shop, StepFailure, Asynchronous>,
    ) -> Result<Option<FailWorkflow>, Error> {
        consult(&mut got).await?;
        decided(self.planned(got.attempt()).on_step_failure)
    }
}

impl AsyncOnStepRetry<Shop> for Verdict {
    fn needs() -> HookNeeds<StepRetry> {
        consulting()
    }

    async fn on_step_retry(
        &self,
        mut got: Requested<'_, Shop, StepRetry, Asynchronous>,
    ) -> Result<Option<FailWorkflow>, Error> {
        consult(&mut got).await?;
        decided(self.planned(got.attempt()).on_step_retry)
    }
}

impl AsyncOnStepAbnormalTermination<Shop> for Verdict {
    fn needs() -> HookNeeds<StepAbnormalTermination> {
        consulting()
    }

    async fn on_step_abnormal_termination(
        &self,
        mut got: Requested<'_, Shop, StepAbnormalTermination, Asynchronous>,
    ) -> Result<Option<FailWorkflow>, Error> {
        consult(&mut got).await?;
        decided(self.planned(got.attempt()).on_step_abnormal_termination)
    }
}

impl AsyncOnWorkflowSuccess<Shop> for Closing {
    fn needs() -> HookNeeds<WorkflowSuccess> {
        consulting()
    }

    async fn on_workflow_success(
        &self,
        mut got: Requested<'_, Shop, WorkflowSuccess, Asynchronous>,
    ) -> Result<(), Error> {
        consult(&mut got).await?;
        closed(self.on_workflow_success_fails)
    }
}

impl AsyncOnWorkflowFailure<Shop> for Closing {
    fn needs() -> HookNeeds<WorkflowFailure> {
        consulting()
    }

    async fn on_workflow_failure(
        &self,
        mut got: Requested<'_, Shop, WorkflowFailure, Asynchronous>,
    ) -> Result<(), Error> {
        consult(&mut got).await?;
        closed(self.on_workflow_failure_fails)
    }
}

/// What every asynchronous hook does before returning: it contributes, and reports that it was
/// consulted.
async fn consult<H: PolicyHookKind>(
    got: &mut Requested<'_, Shop, H, Asynchronous>,
) -> Result<(), Error> {
    got.contributor()?
        .contribute("verdict", BAG_VALUE.to_owned());
    got.reporter()?.info("consulted").await?;
    Ok(())
}
