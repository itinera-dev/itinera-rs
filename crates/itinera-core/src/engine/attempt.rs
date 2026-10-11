//! One attempt of a step: building its policies and the step, with its inputs resolved, running
//! it, and acting on how it ended.

#[cfg(feature = "unstable")]
use std::sync::Arc;

use super::end::End;
use super::inputs::Sources;
use super::{Delivery, Journey, LeftForRescue, Next};
use crate::error::Error;
use crate::event::{EventBody, JourneyAbort};
use crate::instance::WorkflowInstance;
use crate::journey::{Abort, Contributions, Contributor};
use crate::policy::{BuiltStepPolicy, StepHook, StepPolicyEntry};
#[cfg(feature = "unstable")]
use crate::service::Instance;
use crate::step::{
    FromRescued, Got, Outcome, Reporting, StepAttempt, StepDescriptor, StepName, StepRequest,
};
use crate::value::AnyValue;
use policies::build_policies;

mod policies;

/// One attempt of a step once it ended, while its hooks are called: the step, the policies built
/// for the attempt, and what it contributed, committed or not.
pub(super) struct Attempting<'s, W, M> {
    pub(super) step: &'s StepDescriptor<W, M>,
    pub(super) attempt: StepAttempt,
    policies: Vec<Box<dyn BuiltStepPolicy<W, M>>>,
    pub(super) contributed: Contributions,
}

impl<D: Delivery> Journey<D> {
    /// Builds the step's policies and the step for one attempt, runs it, and acts on how it
    /// ended.
    pub(super) async fn attempt<'s, I: WorkflowInstance>(
        &mut self,
        instance: &mut I,
        step: &'s StepDescriptor<I::Workflow, I::Mode>,
        attempt: &StepAttempt,
        mut left: Option<LeftForRescue>,
    ) -> Result<Next<'s, I::Workflow, I::Mode>, End> {
        self.emit(EventBody::AttemptStarted {
            step: attempt.clone(),
        })
        .await?;
        let policies = build_policies(step)?;
        let sources = Sources {
            adapter: instance.descriptor().adapter(step.name()),
            workflow: instance.workflow(),
            journey_id: instance.journey_id(),
            data_bag: instance.data_bag(),
        };
        let mut inputs = Vec::new();
        let mut rescued = FromRescued::default();
        for request in step.needs().requests() {
            match request {
                StepRequest::Input(input) => {
                    let value = self.input(&sources, attempt, input).await?;
                    inputs.push((input.key(), value));
                }
                StepRequest::Rescued(request) => {
                    self.rescue_request(&mut rescued, attempt, request, left.as_mut())
                        .await?;
                }
            }
        }
        let mut contributed = Contributions::default();
        let ran = {
            #[cfg(feature = "unstable")]
            let kept = Arc::clone(&self.kept);
            #[cfg(feature = "unstable")]
            let provided = self
                .provide(instance.descriptor(), &sources, &kept, step, attempt)
                .await?;
            let building = Building {
                attempt,
                inputs,
                rescued,
                #[cfg(feature = "unstable")]
                services: provided.lent(),
            };
            self.build_and_run(step, building, &mut contributed).await
        };
        if let Some(aborted) = self.interrupted.take() {
            return Err(End::Aborted(aborted));
        }
        let ran = ran?;
        let attempting = Attempting {
            step,
            attempt: attempt.clone(),
            policies,
            contributed: kept(&ran, contributed),
        };
        match ran {
            Ok(outcome) => self.conclude(instance, &attempting, outcome.into()).await,
            Err(error) => self.terminate(instance, &attempting, error).await,
        }
    }

    /// Builds the step for its attempt, with what was resolved for it and the handles it
    /// declares, and runs it.
    async fn build_and_run<W, M>(
        &mut self,
        step: &StepDescriptor<W, M>,
        building: Building<'_>,
        contributions: &mut Contributions,
    ) -> Result<Result<Outcome, Error>, End> {
        let needs = step.needs();
        let contributor = if needs.wants_contributor() {
            Some(Contributor::new(contributions))
        } else {
            None
        };
        let reporting = if needs.wants_reporter() {
            Some(Reporting::new(self, building.attempt.clone()))
        } else {
            None
        };
        let got = Got::new(building.inputs, building.rescued, contributor, reporting);
        #[cfg(feature = "unstable")]
        let got = got.lending(building.services);
        match step.attempt(got) {
            Ok(running) => Ok(running.await),
            Err(error) => Err(could_not_build(step.name(), error)),
        }
    }
}

/// An attempt about to be built, with what was resolved for it: the step's inputs, in the order
/// it declares them, absent ones as `None`, what a rescue step requested from the step it
/// rescues, and the services lent to it.
struct Building<'s> {
    attempt: &'s StepAttempt,
    inputs: Vec<(&'static str, Option<AnyValue>)>,
    rescued: FromRescued,
    #[cfg(feature = "unstable")]
    services: Vec<(&'static str, &'s Instance)>,
}

/// A policy attached to a step, with its instance built for the attempt.
type Defining<'a, W, M> = (
    &'a (dyn StepPolicyEntry<W, M> + 'static),
    &'a (dyn BuiltStepPolicy<W, M> + 'static),
);

impl<W, M> Attempting<'_, W, M> {
    /// The policy attached to the step that defines `hook`, if one does, with its instance built
    /// for this attempt.
    pub(super) fn defining(&self, hook: StepHook) -> Option<Defining<'_, W, M>> {
        self.step
            .policies()
            .iter()
            .map(Box::as_ref)
            .zip(self.policies.iter().map(Box::as_ref))
            .find(|(entry, _)| entry.defines(hook))
    }
}

/// What an attempt that ended this way contributed: nothing, after an abnormal termination.
fn kept(ran: &Result<Outcome, Error>, contributed: Contributions) -> Contributions {
    match ran {
        Ok(_) => contributed,
        Err(_) => Contributions::default(),
    }
}

pub(super) fn could_not_build(step: StepName, error: Error) -> End {
    let reported = JourneyAbort::StepCouldNotBeBuilt {
        step,
        #[cfg(feature = "unstable")]
        provider: None,
        error: error.to_string(),
    };
    End::aborted(Abort::StepCouldNotBeBuilt(error), reported)
}

#[cfg(test)]
mod tests;
