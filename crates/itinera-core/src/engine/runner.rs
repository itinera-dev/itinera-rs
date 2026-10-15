//! Runners: what the journey hands an attempt once everything it needs is resolved, to build its
//! step and run it, or, in a test's scripted run, to do what the script says instead.

use super::Emitting;
use crate::error::Error;
use crate::journey::Contributor;
use crate::service::Instance;
use crate::step::{Attempts, FromRescued, Got, Outcome, Reporting, StepAttempt, StepNeeds};
use crate::value::AnyValue;

/// Builds the step of an attempt and runs it, or does what the attempt does instead.
#[async_trait::async_trait]
pub(crate) trait Runner: Send + Sync {
    /// Runs the attempt: an error building its step, or once it ends, its outcome or the error
    /// that ended it abnormally.
    async fn run<'a>(&'a self, attempt: Attempt<'a>) -> Result<Result<Outcome, Error>, Error>;
}

/// An attempt ready to run: its step's factory and needs, what was resolved for it, where its
/// contributions go, and what its reporter emits through.
pub(crate) struct Attempt<'a> {
    pub(crate) factory: &'a dyn Attempts,
    pub(crate) needs: &'a StepNeeds,
    pub(crate) building: Building<'a>,
    pub(crate) contributions: &'a mut Contributor,
    pub(crate) emitting: &'a mut dyn Emitting,
}

/// What was resolved for an attempt: the step's inputs, in the order it declares them, absent
/// ones as `None`, what a rescue step requested from the step it rescues, and the services lent
/// to it.
pub(crate) struct Building<'s> {
    pub(crate) attempt: &'s StepAttempt,
    pub(crate) inputs: Vec<(&'static str, Option<AnyValue>)>,
    pub(crate) rescued: FromRescued,
    pub(crate) services: Vec<(&'static str, &'s Instance)>,
}

/// Runs each attempt's step: builds it with what was resolved and the handles it declares, and
/// runs it.
pub(crate) struct Steps;

#[async_trait::async_trait]
impl Runner for Steps {
    async fn run<'a>(&'a self, attempt: Attempt<'a>) -> Result<Result<Outcome, Error>, Error> {
        let got = lend(
            attempt.needs,
            attempt.building,
            attempt.contributions,
            attempt.emitting,
        );
        attempt.factory.attempt(got).await
    }
}

/// What a factory builds the attempt's step from: what was resolved, with a contributor and a
/// reporter when the step declares them.
pub(crate) fn lend<'a>(
    needs: &StepNeeds,
    building: Building<'a>,
    contributions: &'a mut Contributor,
    emitting: &'a mut dyn Emitting,
) -> Got<'a> {
    let contributor = if needs.wants_contributor() {
        Some(contributions)
    } else {
        None
    };
    let reporting = if needs.wants_reporter() {
        Some(Reporting::new(emitting, building.attempt.clone()))
    } else {
        None
    };
    Got::new(building.inputs, building.rescued, contributor, reporting).lending(building.services)
}
