//! The scenario's scripted policies, declared in the mode of the executor that runs them.

use itinera::error::Error;
use itinera::mode::Asynchronous;
use itinera::policy::{
    AsyncOnStepAbnormalTermination, AsyncOnStepFailure, AsyncOnStepRetry, AsyncOnStepSuccess,
    AsyncOnWorkflowFailure, AsyncOnWorkflowSuccess, FailWorkflow, OnStepAbnormalTermination,
    OnStepFailure, OnStepRetry, OnStepSuccess, OnSuccess, OnWorkflowFailure, OnWorkflowSuccess,
    Requested, StepAbnormalTermination, StepFailure, StepRetry, StepSuccess, WorkflowFailure,
    WorkflowSuccess,
};

use crate::declaration::ScriptedWorkflow;

/// A policy of the scenario, of either kind, whose hooks return nothing.
#[derive(Debug)]
pub(crate) struct ScriptedPolicy;

impl OnStepSuccess<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_success(
        &self,
        _got: Requested<'_, ScriptedWorkflow, StepSuccess>,
    ) -> Result<Option<OnSuccess>, Error> {
        Ok(None)
    }
}

impl OnStepFailure<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_failure(
        &self,
        _got: Requested<'_, ScriptedWorkflow, StepFailure>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(None)
    }
}

impl OnStepRetry<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_retry(
        &self,
        _got: Requested<'_, ScriptedWorkflow, StepRetry>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(None)
    }
}

impl OnStepAbnormalTermination<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_abnormal_termination(
        &self,
        _got: Requested<'_, ScriptedWorkflow, StepAbnormalTermination>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(None)
    }
}

impl OnWorkflowSuccess<ScriptedWorkflow> for ScriptedPolicy {
    fn on_workflow_success(
        &self,
        _got: Requested<'_, ScriptedWorkflow, WorkflowSuccess>,
    ) -> Result<(), Error> {
        Ok(())
    }
}

impl OnWorkflowFailure<ScriptedWorkflow> for ScriptedPolicy {
    fn on_workflow_failure(
        &self,
        _got: Requested<'_, ScriptedWorkflow, WorkflowFailure>,
    ) -> Result<(), Error> {
        Ok(())
    }
}

impl AsyncOnStepSuccess<ScriptedWorkflow> for ScriptedPolicy {
    async fn on_step_success(
        &self,
        _got: Requested<'_, ScriptedWorkflow, StepSuccess, Asynchronous>,
    ) -> Result<Option<OnSuccess>, Error> {
        Ok(None)
    }
}

impl AsyncOnStepFailure<ScriptedWorkflow> for ScriptedPolicy {
    async fn on_step_failure(
        &self,
        _got: Requested<'_, ScriptedWorkflow, StepFailure, Asynchronous>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(None)
    }
}

impl AsyncOnStepRetry<ScriptedWorkflow> for ScriptedPolicy {
    async fn on_step_retry(
        &self,
        _got: Requested<'_, ScriptedWorkflow, StepRetry, Asynchronous>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(None)
    }
}

impl AsyncOnStepAbnormalTermination<ScriptedWorkflow> for ScriptedPolicy {
    async fn on_step_abnormal_termination(
        &self,
        _got: Requested<'_, ScriptedWorkflow, StepAbnormalTermination, Asynchronous>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(None)
    }
}

impl AsyncOnWorkflowSuccess<ScriptedWorkflow> for ScriptedPolicy {
    async fn on_workflow_success(
        &self,
        _got: Requested<'_, ScriptedWorkflow, WorkflowSuccess, Asynchronous>,
    ) -> Result<(), Error> {
        Ok(())
    }
}

impl AsyncOnWorkflowFailure<ScriptedWorkflow> for ScriptedPolicy {
    async fn on_workflow_failure(
        &self,
        _got: Requested<'_, ScriptedWorkflow, WorkflowFailure, Asynchronous>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
