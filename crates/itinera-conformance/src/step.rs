//! The scenario's scripted steps: what each attempt of a step does, as the scenario says.

mod alone;
mod attempt;
mod attempt_end;
mod building;
mod input;
mod reporter;
mod rescue;
mod script;
mod service;

use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::mode::Asynchronous;
use itinera::step::{
    AsyncStepFactory, AsyncStepReporter, Resolved, StepFactory, StepNeeds, StepReporter,
};
use itinera::value::Value as Storable;

pub(crate) use reporter::Emitted;

use attempt::Attempting;
use building::Builds;
use input::taken;
use script::Script;

use crate::model::{self, ModelError};
use crate::value::{Takes, Typed};
use crate::witness::{Build, Rescued, Witness};

/// How many times the scenario's steps ran, whatever step it was.
#[derive(Clone, Debug, Default)]
pub(crate) struct StepsRun {
    count: Arc<AtomicUsize>,
}

impl StepsRun {
    pub(crate) fn none(&self) -> bool {
        self.count.load(Ordering::SeqCst) == 0
    }

    fn one_more(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

/// A step's factory, which builds the step for each attempt to do what its script says, and
/// counts its runs in the scenario's `StepsRun`.
#[derive(Debug)]
pub(crate) struct Scripted {
    name: String,
    script: Script,
    /// How many attempts it was built for, in this journey.
    attempts: AtomicUsize,
    runs: StepsRun,
    witness: Witness,
}

impl Scripted {
    /// The factory of the step `name` with this script, whose runs count in `runs` and whose
    /// builds the witness records.
    pub(crate) fn new(
        name: &str,
        step: &model::Step,
        runs: StepsRun,
        witness: Witness,
    ) -> Result<Self, ModelError> {
        Ok(Self {
            name: name.to_owned(),
            script: Script::of(step)?,
            attempts: AtomicUsize::new(0),
            runs,
            witness,
        })
    }

    /// The factory of the rescue step `name` with this script, which cannot skip.
    pub(crate) fn rescue(
        name: &str,
        step: &model::Step,
        runs: StepsRun,
        witness: Witness,
    ) -> Result<Self, ModelError> {
        let scripted = Self::new(name, step, runs, witness)?;
        if scripted.script.skips() {
            Err(ModelError::RescueSkips(name.to_owned()))
        } else {
            Ok(scripted)
        }
    }

    /// The step for the next attempt, with the reporter of the workflow's mode, recording what it
    /// was built with, including what a rescue step received from the step it rescues.
    fn attempting<'a, G: Builds<'a>>(
        &'a self,
        got: &mut G,
        rescued: Rescued,
    ) -> Result<Attempting<'a, G::Reporter>, Error> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        if let Some(message) = &self.script.construction_failure {
            return Err(Error::msg(message.clone()));
        }
        let inputs = self
            .script
            .inputs
            .iter()
            .map(|input| taken(got, input))
            .collect::<Result<_, _>>()?;
        let services = self
            .script
            .services
            .iter()
            .map(|token| token.lent(got))
            .collect::<Result<_, _>>()?;
        let number = counted(attempt);
        self.witness.built(Build {
            step: self.name.clone(),
            attempt: number,
            inputs,
            rescued,
        });
        Ok(Attempting {
            step: &self.name,
            attempt: number,
            services,
            script: &self.script,
            end: self.script.end(attempt),
            contributor: self.script.declared(|| got.contributor())?,
            reporter: self.script.declared(|| got.reporter())?,
            carries_on: self.script.ignores_failed_emits,
            runs: &self.runs,
            witness: &self.witness,
        })
    }
}

/// The attempt, counted from 1, of the build that found this many builds before it.
fn counted(builds_before: usize) -> NonZeroU32 {
    NonZeroU32::MIN.saturating_add(u32::try_from(builds_before).unwrap_or(u32::MAX))
}

impl StepFactory for Scripted {
    type Step<'a> = Attempting<'a, StepReporter<'a>>;

    fn needs(&self) -> StepNeeds {
        self.script.needs()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Self::Step<'a>, Error> {
        self.attempting(got, Rescued::default())
    }
}

impl AsyncStepFactory for Scripted {
    type Step<'a> = Attempting<'a, AsyncStepReporter<'a>>;

    fn needs(&self) -> StepNeeds {
        self.script.needs()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a, Asynchronous>) -> Result<Self::Step<'a>, Error> {
        self.attempting(got, Rescued::default())
    }
}

pub(crate) fn contribute(contributor: &mut Contributor<'_>, key: &str, value: &Typed) {
    value.clone().given_to(Contributing { contributor, key });
}

struct Contributing<'c, 'a> {
    contributor: &'c mut Contributor<'a>,
    key: &'c str,
}

impl Takes for Contributing<'_, '_> {
    type Output = ();

    fn take<T: Storable>(self, value: T) {
        self.contributor.contribute(self.key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journey::fixtures::attempt;
    use crate::model::AttemptOutcome;

    #[test]
    fn a_rescue_step_that_skips_is_a_case_error() {
        let step = model::Step {
            attempts: Some(vec![attempt(AttemptOutcome::Skipped(None))]),
            ..model::Step::default()
        };

        let scripted = Scripted::rescue(
            "charge_rescue",
            &step,
            StepsRun::default(),
            Witness::default(),
        );

        assert_eq!(
            scripted.err(),
            Some(ModelError::RescueSkips("charge_rescue".to_owned()))
        );
    }
}
