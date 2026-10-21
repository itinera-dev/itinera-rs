//! The mistakes in a test that test support stops it for, read from what itinera reports, in
//! the words the cases name them with.

mod reported;

use std::str::FromStr;

use crate::model::{ModelError, Row};

pub(crate) use reported::mistakes;

/// A mistake in a test, as a row of "the test stopped before the journey, naming:" writes it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Mistake {
    pub(crate) mistake: MistakeName,
    pub(crate) step: Option<String>,
    /// The attempt concerned, which may be one the step can never have, such as attempt 0.
    pub(crate) attempt: Option<u32>,
    pub(crate) provider: Option<String>,
}

/// The names the cases give the mistakes in a test.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum MistakeName {
    AttemptLeftOut,
    RescueStepLeftOut,
    StepLeftOut,
    UnknownStep,
    StepScriptedTwice,
    GapInTheAttempts,
    AttemptScriptedTwice,
    NoSuchAttempt,
    RescueStepSkips,
    NoContributor,
    NoReporter,
    UnknownProvider,
    AnotherService,
}

impl FromStr for MistakeName {
    type Err = ModelError;

    fn from_str(name: &str) -> Result<Self, ModelError> {
        Ok(match name {
            "attempt left out" => Self::AttemptLeftOut,
            "rescue step left out" => Self::RescueStepLeftOut,
            "step left out" => Self::StepLeftOut,
            "unknown step" => Self::UnknownStep,
            "step scripted twice" => Self::StepScriptedTwice,
            "gap in the attempts" => Self::GapInTheAttempts,
            "attempt scripted twice" => Self::AttemptScriptedTwice,
            "no such attempt" => Self::NoSuchAttempt,
            "rescue step skips" => Self::RescueStepSkips,
            "no contributor" => Self::NoContributor,
            "no reporter" => Self::NoReporter,
            "unknown provider" => Self::UnknownProvider,
            "another service" => Self::AnotherService,
            _ => return Err(ModelError::Cell("mistake", name.to_owned())),
        })
    }
}

impl Mistake {
    /// The mistake a row names.
    pub(crate) fn of(row: &Row) -> Result<Self, ModelError> {
        Ok(Self {
            mistake: row.parse("mistake")?,
            step: row.optional("step").map(str::to_owned),
            attempt: row
                .optional("attempt")
                .map(|_| row.parse("attempt"))
                .transpose()?,
            provider: row.optional("provider").map(str::to_owned),
        })
    }

    /// The mistake concerning this step alone.
    pub(crate) fn of_step(mistake: MistakeName, step: &str) -> Self {
        Self {
            mistake,
            step: Some(step.to_owned()),
            attempt: None,
            provider: None,
        }
    }

    fn of_attempt(mistake: MistakeName, step: &str, attempt: &str) -> Option<Self> {
        Some(Self {
            attempt: Some(attempt.parse().ok()?),
            ..Self::of_step(mistake, step)
        })
    }

    fn of_provider(mistake: MistakeName, provider: &str) -> Self {
        Self {
            mistake,
            step: None,
            attempt: None,
            provider: Some(provider.to_owned()),
        }
    }
}
