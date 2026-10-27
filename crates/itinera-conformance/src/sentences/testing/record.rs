//! The record of a scripted run: the attempts handed to each step, and what each read.

use std::num::NonZeroU32;

use cucumber::then;
use itinera::journey::{AttemptRecord, ScriptedRun};
use itinera::value::Value as Storable;
use serde_json::Value;

use super::found;
use crate::model::{ModelError, json};
use crate::sentences::{Unmet, holds, journey, refused};
use crate::testing::{Mistake, MistakeName, mistakes, unless_stopped_reading};
use crate::value::{ForType, Typed};
use crate::world::World;

#[then(
    regex = r#"^the scripted run's record of step "([^"]*)" holds the attempts (\d+(?:, \d+)*)$"#
)]
fn the_record_holds_the_attempts(
    world: &mut World,
    step_name: String,
    numbers: String,
) -> Result<(), Unmet> {
    let expected = numbers
        .split(", ")
        .map(str::parse)
        .collect::<Result<Vec<NonZeroU32>, _>>()
        .map_err(|_| ModelError::Cell("attempts", numbers.clone()))?;
    attempts_are(world, &step_name, &expected)
}

#[then(expr = "the scripted run's record of step {string} holds no attempt")]
fn the_record_holds_no_attempt(world: &mut World, step_name: String) -> Result<(), Unmet> {
    attempts_are(world, &step_name, &[])
}

/// Holds when the record of the step holds exactly the attempts of these numbers, in order.
fn attempts_are(world: &World, step: &str, expected: &[NonZeroU32]) -> Result<(), Unmet> {
    let found: Vec<NonZeroU32> = record(world)?
        .step(step)
        .attempts()
        .iter()
        .map(AttemptRecord::number)
        .map(NonZeroU32::from)
        .collect();
    holds(
        found == expected,
        format_args!("the record of step \"{step}\" to hold the attempts {expected:?}"),
        format_args!("{found:?}"),
    )
}

#[then(
    regex = r#"^attempt (\d+) of step "([^"]*)" in the scripted run's record read the (?:input|rescued step data) "([^"]*)" = (.+)$"#
)]
fn the_attempt_in_the_record_read(
    world: &mut World,
    attempt: u32,
    step_name: String,
    key: String,
    value: String,
) -> Result<(), Unmet> {
    let expected = json(&value)?;
    let record = record(world)?
        .step(&step_name)
        .attempt(attempt)
        .ok_or_else(|| unrecorded(attempt, &step_name))?;
    let reading = Input { record, key: &key };
    let found = Typed::of(&expected)?.for_its_type(reading)?;
    holds(
        found.as_ref() == Some(&expected),
        format_args!("attempt {attempt} of step \"{step_name}\" to read \"{key}\" = {expected}"),
        format_args!("{found:?}"),
    )
}

#[then(
    regex = r#"^reading the scripted run's record of step "([^"]*)" stops the test as the mistake "([^"]*)"$"#
)]
fn reading_the_record_stops_the_test(
    world: &mut World,
    step_name: String,
    mistake: String,
) -> Result<(), Unmet> {
    let expected = vec![Mistake::of_step(
        mistake.parse::<MistakeName>()?,
        &step_name,
    )];
    let run = record(world)?;
    let stopped = unless_stopped_reading(|| run.step(&step_name))
        .err()
        .ok_or_else(|| not_stopped_reading(&step_name))?;
    let named = mistakes(&stopped);
    holds(
        named.as_ref() == Some(&expected),
        format_args!("reading the record to stop the test, naming {expected:?}"),
        format_args!("{named:?}, from \"{stopped}\""),
    )
}

#[then(
    regex = r#"^reading attempt (\d+) of step "([^"]*)" in the scripted run's record reads nothing$"#
)]
fn reading_the_attempt_in_the_record_reads_nothing(
    world: &mut World,
    attempt: u32,
    step_name: String,
) -> Result<(), Unmet> {
    let found = record(world)?
        .step(&step_name)
        .attempt(attempt)
        .map(AttemptRecord::number);
    holds(
        found.is_none(),
        format_args!("attempt {attempt} of step \"{step_name}\" to read nothing"),
        format_args!("{found:?}"),
    )
}

fn not_stopped_reading(step: &str) -> Unmet {
    Unmet::Expected(format!(
        "reading the record of \"{step}\" to stop the test; it did not"
    ))
}

fn unrecorded(attempt: u32, step: &str) -> Unmet {
    Unmet::Expected(format!(
        "attempt {attempt} of step \"{step}\" in the record; it has none"
    ))
}

/// Reads an input of an attempt in the record, as the type of the value expected.
struct Input<'a> {
    record: &'a AttemptRecord,
    key: &'a str,
}

impl ForType for Input<'_> {
    type Output = Result<Option<Value>, serde_json::Error>;

    fn of<T: Storable>(self) -> Self::Output {
        found(self.record.input::<T>(self.key))
    }
}

/// The record of the scripted run of the scenario's last journey.
fn record(world: &World) -> Result<&ScriptedRun, Unmet> {
    journey(world)?
        .run
        .as_ref()
        .map_err(refused)?
        .record()
        .ok_or_else(|| Unmet::Case("the journey ran without a script".to_owned()))
}
