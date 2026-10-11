//! The scenario's scripted steps: what each attempt of a step does, as the scenario says.

mod attempt;
mod ending;
mod input;
mod reporter;
mod script;

use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::mode::{Asynchronous, Synchronous};
use itinera::step::{
    AsyncStepFactory, AsyncStepReporter, Resolved, StepFactory, StepNeeds, StepReporter,
};
use itinera::value::Value as Storable;

pub(crate) use reporter::Emitted;

use attempt::Attempting;
use input::taken;
use script::Script;

use crate::model::{self, ModelError};
use crate::value::{Takes, Typed};
use crate::witness::{Build, Witness};

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

    /// The step for the next attempt, with the reporter of the workflow's mode.
    fn attempting<'a, M, R>(
        &'a self,
        got: &mut Resolved<'a, M>,
        reporter: fn(&mut Resolved<'a, M>) -> Result<R, Error>,
    ) -> Result<Attempting<'a, R>, Error> {
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
        self.witness.built(Build {
            step: self.name.clone(),
            attempt: counted(attempt),
            inputs,
        });
        Ok(Attempting {
            script: &self.script,
            ending: self.script.ending(attempt),
            contributor: got.contributor()?,
            reporter: reporter(got)?,
            runs: &self.runs,
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
        self.attempting(got, Resolved::<'a, Synchronous>::reporter)
    }
}

impl AsyncStepFactory for Scripted {
    type Step<'a> = Attempting<'a, AsyncStepReporter<'a>>;

    fn needs(&self) -> StepNeeds {
        self.script.needs()
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a, Asynchronous>) -> Result<Self::Step<'a>, Error> {
        self.attempting(got, Resolved::<'a, Asynchronous>::reporter)
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
