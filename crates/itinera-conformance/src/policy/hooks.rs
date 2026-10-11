//! The hooks of a scripted policy, in both modes: each does what its script says.

use std::future::Future;

use futures::executor::block_on;
use itinera::error::Error;
use itinera::mode::{Asynchronous, Synchronous};
use itinera::policy::{
    AsyncOnStepAbnormalTermination, AsyncOnStepFailure, AsyncOnStepRetry, AsyncOnStepSuccess,
    AsyncOnWorkflowFailure, AsyncOnWorkflowSuccess, FailWorkflow, OnStepAbnormalTermination,
    OnStepFailure, OnStepRetry, OnStepSuccess, OnSuccess, OnWorkflowFailure, OnWorkflowSuccess,
    Requested, StepAbnormalTermination, StepFailure, StepRetry, StepSuccess, WorkflowFailure,
    WorkflowSuccess,
};

use super::ScriptedPolicy;
use crate::declaration::ScriptedWorkflow;

impl OnStepSuccess<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_success(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepSuccess>,
    ) -> Result<Option<OnSuccess>, Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl OnStepFailure<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_failure(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepFailure>,
    ) -> Result<Option<FailWorkflow>, Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl OnStepRetry<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_retry(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepRetry>,
    ) -> Result<Option<FailWorkflow>, Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl OnStepAbnormalTermination<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_abnormal_termination(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepAbnormalTermination>,
    ) -> Result<Option<FailWorkflow>, Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl OnWorkflowSuccess<ScriptedWorkflow> for ScriptedPolicy {
    fn on_workflow_success(
        &self,
        got: Requested<'_, ScriptedWorkflow, WorkflowSuccess>,
    ) -> Result<(), Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl OnWorkflowFailure<ScriptedWorkflow> for ScriptedPolicy {
    fn on_workflow_failure(
        &self,
        got: Requested<'_, ScriptedWorkflow, WorkflowFailure>,
    ) -> Result<(), Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl AsyncOnStepSuccess<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_success(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepSuccess, Asynchronous>,
    ) -> impl Future<Output = Result<Option<OnSuccess>, Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

impl AsyncOnStepFailure<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_failure(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepFailure, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

impl AsyncOnStepRetry<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_retry(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepRetry, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

impl AsyncOnStepAbnormalTermination<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_abnormal_termination(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepAbnormalTermination, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

impl AsyncOnWorkflowSuccess<ScriptedWorkflow> for ScriptedPolicy {
    fn on_workflow_success(
        &self,
        got: Requested<'_, ScriptedWorkflow, WorkflowSuccess, Asynchronous>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

impl AsyncOnWorkflowFailure<ScriptedWorkflow> for ScriptedPolicy {
    fn on_workflow_failure(
        &self,
        got: Requested<'_, ScriptedWorkflow, WorkflowFailure, Asynchronous>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}
