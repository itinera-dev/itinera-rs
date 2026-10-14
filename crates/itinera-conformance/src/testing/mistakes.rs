//! The mistakes in a test that test support stops it for, read from what itinera reports, in
//! the words the cases name them with.

use std::num::NonZeroU32;
use std::str::FromStr;

use crate::model::{ModelError, Row};

/// A mistake in a test, as a row of "the test stopped before the journey, naming:" writes it.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Mistake {
    pub(crate) mistake: MistakeName,
    pub(crate) step: Option<String>,
    pub(crate) attempt: Option<NonZeroU32>,
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

    fn of_step(mistake: MistakeName, step: &str) -> Self {
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

/// The mistakes what itinera reported names, in order, or none when it names something else.
pub(crate) fn mistakes(reported: &str) -> Option<Vec<Mistake>> {
    match reported.strip_prefix("the script leaves out:\n") {
        Some(left_out) => left_out.lines().map(left_out_one).collect(),
        None => Some(vec![written(reported)?]),
    }
}

/// One line of what a script leaves out, which the check before the journey reports.
fn left_out_one(line: &str) -> Option<Mistake> {
    let line = line.strip_prefix("- ")?;
    if let Some(step) = line.strip_suffix(", since every step must be scripted") {
        return Some(Mistake::of_step(MistakeName::StepLeftOut, step));
    }
    if let Some((rescue, _)) = line.split_once(", since ") {
        return Some(Mistake::of_step(MistakeName::RescueStepLeftOut, rescue));
    }
    let (attempt, step) = line.strip_prefix("attempt ")?.split_once(" of ")?;
    Mistake::of_attempt(MistakeName::AttemptLeftOut, step, attempt)
}

/// A mistake reported as the test writes it.
fn written(reported: &str) -> Option<Mistake> {
    [
        unknown_name,
        gap,
        scripted_twice,
        rescue_skips,
        undeclared_handle,
        another_service,
    ]
    .into_iter()
    .find_map(|reading| reading(reported))
}

/// A step or a provider the workflow does not have.
fn unknown_name(reported: &str) -> Option<Mistake> {
    let (_, named) = reported
        .strip_prefix("the workflow ")?
        .split_once(" has no ")?;
    match named.split_once(" named ")? {
        ("step", step) => Some(Mistake::of_step(MistakeName::UnknownStep, step)),
        ("service provider", provider) => {
            Some(Mistake::of_provider(MistakeName::UnknownProvider, provider))
        }
        _ => None,
    }
}

fn gap(reported: &str) -> Option<Mistake> {
    let (_, rest) = reported.strip_prefix("attempt ")?.split_once(" of ")?;
    let (step, attempt) = rest.split_once(" is missing before attempt ")?;
    Mistake::of_attempt(MistakeName::GapInTheAttempts, step, attempt)
}

/// A step named twice; an attempt scripted twice is another mistake.
fn scripted_twice(reported: &str) -> Option<Mistake> {
    let step = reported.strip_suffix(" is scripted already")?;
    (!step.contains(' ')).then(|| Mistake::of_step(MistakeName::StepScriptedTwice, step))
}

fn rescue_skips(reported: &str) -> Option<Mistake> {
    let step = reported.strip_suffix(" is a rescue step, which cannot skip")?;
    Some(Mistake::of_step(MistakeName::RescueStepSkips, step))
}

fn undeclared_handle(reported: &str) -> Option<Mistake> {
    let (step, rest) = reported.split_once(" declares no ")?;
    match rest.split_once(", so it cannot ")? {
        ("contributor", _) => Some(Mistake::of_step(MistakeName::NoContributor, step)),
        ("reporter", _) => Some(Mistake::of_step(MistakeName::NoReporter, step)),
        _ => None,
    }
}

fn another_service(reported: &str) -> Option<Mistake> {
    let (provider, _) = reported
        .strip_prefix("the fake of the service provider ")?
        .split_once(" supplies ")?;
    Some(Mistake::of_provider(MistakeName::AnotherService, provider))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn what_a_script_leaves_out_names_each_mistake() {
        let reported = "the script leaves out:\n- attempt 2 of charge\n- charge_rescue, since \
                        charge can fail on attempt 1\n- ship, since every step must be scripted";
        assert_eq!(
            mistakes(reported),
            Some(vec![
                Mistake {
                    attempt: NonZeroU32::new(2),
                    ..Mistake::of_step(MistakeName::AttemptLeftOut, "charge")
                },
                Mistake::of_step(MistakeName::RescueStepLeftOut, "charge_rescue"),
                Mistake::of_step(MistakeName::StepLeftOut, "ship"),
            ])
        );
    }

    #[rstest]
    #[case::unknown_step(
        "the workflow orders has no step named refund",
        MistakeName::UnknownStep,
        "refund"
    )]
    #[case::step_scripted_twice(
        "charge is scripted already",
        MistakeName::StepScriptedTwice,
        "charge"
    )]
    #[case::rescue_step_skips(
        "charge_rescue is a rescue step, which cannot skip",
        MistakeName::RescueStepSkips,
        "charge_rescue"
    )]
    #[case::no_contributor(
        "charge declares no contributor, so it cannot contribute",
        MistakeName::NoContributor,
        "charge"
    )]
    #[case::no_reporter(
        "charge declares no reporter, so it cannot set global metadata",
        MistakeName::NoReporter,
        "charge"
    )]
    fn a_mistake_in_writing_the_script_names_its_step(
        #[case] reported: &str,
        #[case] mistake: MistakeName,
        #[case] step: &str,
    ) {
        assert_eq!(
            mistakes(reported),
            Some(vec![Mistake::of_step(mistake, step)])
        );
    }

    #[test]
    fn a_gap_names_the_attempt_written_too_early() {
        assert_eq!(
            mistakes("attempt 2 of charge is missing before attempt 3"),
            Some(vec![Mistake {
                attempt: NonZeroU32::new(3),
                ..Mistake::of_step(MistakeName::GapInTheAttempts, "charge")
            }])
        );
    }

    #[rstest]
    #[case::unknown_provider(
        "the workflow orders has no service provider named paypal",
        MistakeName::UnknownProvider,
        "paypal"
    )]
    #[case::another_service(
        "the fake of the service provider payments supplies Meter, but the provider supplies \
         Labeller",
        MistakeName::AnotherService,
        "payments"
    )]
    fn a_mistake_in_a_fake_names_its_provider(
        #[case] reported: &str,
        #[case] mistake: MistakeName,
        #[case] provider: &str,
    ) {
        assert_eq!(
            mistakes(reported),
            Some(vec![Mistake::of_provider(mistake, provider)])
        );
    }

    #[rstest]
    #[case::an_attempt_scripted_twice("attempt 1 of charge is scripted already")]
    #[case::a_bug("called `Option::unwrap()` on a `None` value")]
    fn anything_else_names_no_mistake(#[case] reported: &str) {
        assert_eq!(mistakes(reported), None);
    }
}
