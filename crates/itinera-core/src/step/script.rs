//! Scripts for tests: what the attempts of a step do instead of running it.

use std::num::NonZeroU32;

use super::{Attempt, Ending, StepAttempt, StepName};
use crate::event::Level;
use crate::value::AnyValue;

/// What the scripted steps of a journey do, step by step.
#[derive(Debug, Default, derive_more::From)]
pub(crate) struct Script {
    steps: Vec<(StepName, StepScript)>,
}

impl Script {
    /// What the attempt does instead of running the step, unless the step runs for real.
    pub(crate) fn played(&self, attempt: &StepAttempt) -> Option<&Played> {
        self.steps
            .iter()
            .find(|entry| is_for(entry, attempt.step))
            .and_then(|(_, script)| script.attempt(attempt.attempt))
            .and_then(AttemptScript::played)
    }
}

fn is_for((name, _): &&(StepName, StepScript), step: StepName) -> bool {
    *name == step
}

/// What a step's attempts do: those numbered from 1, in order, then every attempt after them.
#[derive(Debug, Default)]
pub(crate) struct StepScript {
    numbered: Vec<AttemptScript>,
    remaining: Option<AttemptScript>,
}

impl StepScript {
    /// What the attempt does, if the script says.
    pub(crate) fn attempt(&self, attempt: Attempt) -> Option<&AttemptScript> {
        usize::try_from(NonZeroU32::from(attempt).get() - 1)
            .ok()
            .and_then(|index| self.numbered.get(index))
            .or(self.remaining.as_ref())
    }

    /// How many attempts it numbers.
    pub(crate) fn numbered(&self) -> u32 {
        u32::try_from(self.numbered.len()).unwrap_or(u32::MAX)
    }

    /// Whether it says what every attempt after the numbered ones does.
    pub(crate) fn covers_the_rest(&self) -> bool {
        self.remaining.is_some()
    }

    /// Adds the next numbered attempt.
    pub(crate) fn push(&mut self, attempt: AttemptScript) {
        self.numbered.push(attempt);
    }

    /// Says what every attempt after the numbered ones does.
    pub(crate) fn cover_the_rest(&mut self, attempt: AttemptScript) {
        self.remaining = Some(attempt);
    }
}

/// What one attempt does: run the real step, or play what the script says instead.
#[derive(Debug)]
pub(crate) enum AttemptScript {
    RealRun,
    Played(Played),
}

impl AttemptScript {
    fn played(&self) -> Option<&Played> {
        match self {
            Self::RealRun => None,
            Self::Played(played) => Some(played),
        }
    }
}

/// What a scripted attempt does instead of running the step: its actions, in order, then its
/// ending.
#[derive(Clone, Debug)]
pub(crate) struct Played {
    pub(crate) actions: Vec<Action>,
    pub(crate) ending: Ending,
}

/// Something a scripted attempt does before it ends.
#[derive(Clone, Debug)]
pub(crate) enum Action {
    Contribute {
        key: String,
        value: AnyValue,
    },
    Emit {
        level: Level,
        message: String,
        data: Option<AnyValue>,
    },
    SetGlobalMetadata {
        key: String,
        value: AnyValue,
    },
}
