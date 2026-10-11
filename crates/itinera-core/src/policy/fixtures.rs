//! Fixtures the tests of several modules share: a policy whose hooks change nothing, and the
//! keys of the data a hook needs.

#[cfg(feature = "async")]
use super::AsyncOnWorkflowSuccess;
use super::{
    FailWorkflow, Needs, OnStepFailure, OnStepSuccess, OnSuccess, OnWorkflowFailure,
    OnWorkflowSuccess, Request, Requested, StepFailure, StepSuccess, WorkflowFailure,
    WorkflowSuccess,
};
use crate::error::Error;
use crate::step::Input;

pub(super) const RECEIPT: Input<String> = Input::new("receipt");

/// A policy whose hooks change nothing.
pub(crate) struct Quiet;

impl<W: Send + Sync + 'static> OnStepSuccess<W> for Quiet {
    fn on_step_success(
        &self,
        _: Requested<'_, W, StepSuccess>,
    ) -> Result<Option<OnSuccess>, Error> {
        Ok(None)
    }
}

impl<W: Send + Sync + 'static> OnStepFailure<W> for Quiet {
    fn on_step_failure(
        &self,
        _: Requested<'_, W, StepFailure>,
    ) -> Result<Option<FailWorkflow>, Error> {
        Ok(None)
    }
}

impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Quiet {
    fn on_workflow_success(&self, _: Requested<'_, W, WorkflowSuccess>) -> Result<(), Error> {
        Ok(())
    }
}

impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Quiet {
    fn on_workflow_failure(&self, _: Requested<'_, W, WorkflowFailure>) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(feature = "async")]
impl<W: Send + Sync + 'static> AsyncOnWorkflowSuccess<W> for Quiet {
    async fn on_workflow_success(
        &self,
        _: Requested<'_, W, WorkflowSuccess, crate::mode::Asynchronous>,
    ) -> Result<(), Error> {
        Ok(())
    }
}

/// The keys of the data a hook needs, from the step or from the workflow.
pub(super) fn keys(needs: &Needs) -> Vec<&'static str> {
    needs.requests().iter().filter_map(key).collect()
}

fn key(request: &Request) -> Option<&'static str> {
    match request {
        Request::FromStep(need) | Request::FromWorkflow(need) => Some(need.key()),
        Request::Reason(_) | Request::Error(_) => None,
    }
}
