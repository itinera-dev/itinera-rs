//! The parts of a generated workflow, each doing what its plan says: the steps, the step policy,
//! the workflow policy and a reporter that may fail.

use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use itinera::error::Error;
use itinera::event::Event;
use itinera::journey::Contributor;
use itinera::policy::{
    HookNeeds, OnFailure, OnStepAbnormalTermination, OnStepFailure, OnStepRetry, OnStepSuccess,
    OnSuccess, OnWorkflowFailure, OnWorkflowSuccess, PolicyHookKind, Requested,
    StepAbnormalTermination, StepFailure, StepRetry, StepSuccess, WorkflowFailure, WorkflowSuccess,
};
use itinera::report::Reporter;
use itinera::step::{Outcome, Resolved, Step, StepFactory, StepNeeds, StepReporter};

use crate::journey::{BAG_VALUE, Shop};
use crate::plan::{Attempt, Closing, Ending, OnSuccessReturns, Returns, StepPlan};

#[cfg(feature = "async")]
mod asynchronous;

/// Builds each attempt of a step from the step's plan, counting the attempts it builds, since a
/// step is never told which attempt it is.
pub(crate) struct Scripted {
    attempts: Vec<Attempt>,
    built: AtomicUsize,
}

impl Scripted {
    pub(crate) fn new(step: &StepPlan) -> Self {
        Self {
            attempts: step.attempts.clone(),
            built: AtomicUsize::new(0),
        }
    }

    fn next_ending(&self) -> Ending {
        self.attempts[self.built.fetch_add(1, Ordering::Relaxed)].ending
    }
}

/// One attempt of a scripted step: it contributes, reports that it is working, then ends as
/// planned.
pub(crate) struct ScriptedStep<'a> {
    ending: Ending,
    contributor: Contributor<'a>,
    reporter: StepReporter<'a>,
}

impl StepFactory for Scripted {
    type Step<'a> = ScriptedStep<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new().contributor().reporter()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<ScriptedStep<'a>, Error> {
        Ok(ScriptedStep {
            ending: self.next_ending(),
            contributor: got.contributor()?,
            reporter: got.reporter()?,
        })
    }
}

impl Step for ScriptedStep<'_> {
    fn run(mut self) -> Result<Outcome, Error> {
        self.contributor.contribute("receipt", BAG_VALUE.to_owned());
        self.reporter.info("working")?;
        ended(self.ending)
    }
}

fn ended(ending: Ending) -> Result<Outcome, Error> {
    match ending {
        Ending::Success => Ok(Outcome::success()),
        Ending::Failure => Ok(Outcome::failure("declined")),
        Ending::Skipped => Ok(Outcome::skipped()),
        Ending::AbnormalTermination => Err(Error::msg("the step crashed")),
    }
}

/// The step policy, built for every attempt: each of its hooks is consulted, then returns what
/// the plan of the attempt it acts on says.
pub(crate) struct Verdict {
    attempts: Arc<[Attempt]>,
}

impl Verdict {
    pub(crate) fn new(attempts: &Arc<[Attempt]>) -> Self {
        Self {
            attempts: Arc::clone(attempts),
        }
    }

    fn planned(&self, attempt: NonZeroU32) -> Attempt {
        self.attempts[usize::try_from(attempt.get()).unwrap() - 1]
    }
}

impl OnStepSuccess<Shop> for Verdict {
    fn needs() -> HookNeeds<StepSuccess> {
        consulting()
    }

    fn on_step_success(
        &self,
        mut got: Requested<'_, Shop, StepSuccess>,
    ) -> Result<OnSuccess, Error> {
        consult(&mut got)?;
        on_success(self.planned(got.attempt()).on_step_success)
    }
}

impl OnStepFailure<Shop> for Verdict {
    fn needs() -> HookNeeds<StepFailure> {
        consulting()
    }

    fn on_step_failure(
        &self,
        mut got: Requested<'_, Shop, StepFailure>,
    ) -> Result<OnFailure, Error> {
        consult(&mut got)?;
        decided(self.planned(got.attempt()).on_step_failure)
    }
}

impl OnStepRetry<Shop> for Verdict {
    fn needs() -> HookNeeds<StepRetry> {
        consulting()
    }

    fn on_step_retry(&self, mut got: Requested<'_, Shop, StepRetry>) -> Result<OnFailure, Error> {
        consult(&mut got)?;
        decided(self.planned(got.attempt()).on_step_retry)
    }
}

impl OnStepAbnormalTermination<Shop> for Verdict {
    fn needs() -> HookNeeds<StepAbnormalTermination> {
        consulting()
    }

    fn on_step_abnormal_termination(
        &self,
        mut got: Requested<'_, Shop, StepAbnormalTermination>,
    ) -> Result<OnFailure, Error> {
        consult(&mut got)?;
        decided(self.planned(got.attempt()).on_step_abnormal_termination)
    }
}

impl OnWorkflowSuccess<Shop> for Closing {
    fn needs() -> HookNeeds<WorkflowSuccess> {
        consulting()
    }

    fn on_workflow_success(
        &self,
        mut got: Requested<'_, Shop, WorkflowSuccess>,
    ) -> Result<(), Error> {
        consult(&mut got)?;
        closed(self.on_workflow_success_fails)
    }
}

impl OnWorkflowFailure<Shop> for Closing {
    fn needs() -> HookNeeds<WorkflowFailure> {
        consulting()
    }

    fn on_workflow_failure(
        &self,
        mut got: Requested<'_, Shop, WorkflowFailure>,
    ) -> Result<(), Error> {
        consult(&mut got)?;
        closed(self.on_workflow_failure_fails)
    }
}

/// What every hook of the generated policies needs: a contributor and a reporter.
fn consulting<H: PolicyHookKind>() -> HookNeeds<H> {
    HookNeeds::new().contributor().reporter()
}

/// What every hook does before returning: it contributes, and reports that it was consulted.
fn consult<H: PolicyHookKind>(got: &mut Requested<'_, Shop, H>) -> Result<(), Error> {
    got.contributor()?
        .contribute("verdict", BAG_VALUE.to_owned());
    got.reporter()?.info("consulted")?;
    Ok(())
}

fn on_success(returns: OnSuccessReturns) -> Result<OnSuccess, Error> {
    match returns {
        OnSuccessReturns::Nothing => Ok(OnSuccess::KeepDefault),
        OnSuccessReturns::FinishWorkflow => Ok(OnSuccess::FinishWorkflow),
        OnSuccessReturns::FailWorkflow => Ok(OnSuccess::fail_workflow("refused")),
        OnSuccessReturns::Error => Err(broken()),
    }
}

fn decided(returns: Returns) -> Result<OnFailure, Error> {
    match returns {
        Returns::Nothing => Ok(OnFailure::KeepDefault),
        Returns::FailWorkflow => Ok(OnFailure::fail_workflow("refused")),
        Returns::Error => Err(broken()),
    }
}

fn closed(fails: bool) -> Result<(), Error> {
    if fails { Err(broken()) } else { Ok(()) }
}

fn broken() -> Error {
    Error::msg("the hook broke")
}

/// A reporter that fails on the event its plan says, if on any.
pub(crate) struct Flaky {
    fails_on: Option<NonZeroUsize>,
    received: usize,
}

impl Flaky {
    pub(crate) fn failing_on(fails_on: Option<NonZeroUsize>) -> Self {
        Self {
            fails_on,
            received: 0,
        }
    }
}

impl Reporter for Flaky {
    fn report(&mut self, _: &Event) -> Result<(), Error> {
        self.received += 1;
        if self.fails_on == NonZeroUsize::new(self.received) {
            return Err(Error::msg("the reporter broke"));
        }
        Ok(())
    }
}
