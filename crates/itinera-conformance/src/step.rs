//! The scenario's scripted steps: what each attempt of a step does, as the scenario says.

use std::future::{Future, ready};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::executor::block_on;
use itinera::error::{Error, Interrupted};
use itinera::journey::Contributor;
use itinera::mode::{Asynchronous, Synchronous};
use itinera::step::{
    AsyncStep, AsyncStepFactory, AsyncStepReporter, Input, OptionalInput, Outcome, Reason,
    Resolved, Step, StepFactory, StepNeeds, StepReporter,
};
use itinera::value::Value as Storable;

use crate::declaration::leaked;
use crate::model::{self, AttemptOutcome, Level, ModelError, StepAction, ValueType};
use crate::value::{ForType, Takes, Typed, for_type};

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
    script: Script,
    /// How many attempts it was built for, in this journey.
    attempts: AtomicUsize,
    runs: StepsRun,
}

/// What a step's script says, with its values typed.
///
/// Rust moves a contributed value into the contribution, and gives a step its own copy of each
/// input, so a step that changes either afterwards changes nothing anyone else sees. The script
/// leaves those changes out.
#[derive(Debug)]
struct Script {
    inputs: Vec<Need>,
    /// What every attempt does before it ends, in order.
    actions: Vec<Action>,
    /// How the attempts before the last stated one end, in order.
    endings: Vec<Ending>,
    /// How the last stated attempt ends, and every attempt after it.
    last: Ending,
    construction_failure: Option<String>,
    ignores_failed_emits: bool,
}

#[derive(Debug)]
struct Need {
    key: &'static str,
    value_type: ValueType,
    optional: bool,
}

#[derive(Debug)]
enum Action {
    Contribute(String, Typed),
    Emit {
        level: Level,
        message: String,
        data: Option<Typed>,
    },
}

/// How one attempt ends, and what it contributes just before.
#[derive(Debug)]
struct Ending {
    contributes: Vec<(String, Typed)>,
    end: End,
}

#[derive(Debug)]
enum End {
    Success,
    Failure(Reason),
    RetriableFailure(Reason),
    Skipped(Option<Reason>),
    /// An error escapes the step, with this message.
    Error(String),
}

impl Scripted {
    /// The factory of a step with this script, whose runs count in `runs`.
    pub(crate) fn new(step: &model::Step, runs: StepsRun) -> Result<Self, ModelError> {
        Ok(Self {
            script: Script::of(step)?,
            attempts: AtomicUsize::new(0),
            runs,
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
        Ok(Attempting {
            script: &self.script,
            ending: self.script.ending(attempt),
            contributor: got.contributor()?,
            reporter: reporter(got)?,
            runs: &self.runs,
        })
    }
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

impl Script {
    fn of(step: &model::Step) -> Result<Self, ModelError> {
        let (endings, last) = match step.attempts.as_deref().and_then(<[_]>::split_last) {
            Some((last, endings)) => (
                endings.iter().map(Ending::of).collect::<Result<_, _>>()?,
                Ending::of(last)?,
            ),
            None => (Vec::new(), Ending::success()),
        };
        Ok(Self {
            inputs: step.inputs.iter().map(Need::of).collect(),
            actions: step
                .actions
                .iter()
                .filter_map(Action::of)
                .collect::<Result<_, _>>()?,
            endings,
            last,
            construction_failure: step.construction_failure.clone(),
            ignores_failed_emits: step.ignores_failed_emits,
        })
    }

    /// Its inputs, and both handles, which it takes whether it uses them or not.
    fn needs(&self) -> StepNeeds {
        self.inputs
            .iter()
            .fold(StepNeeds::new().contributor().reporter(), needing)
    }

    /// How the attempt that comes after `before` others ends.
    fn ending(&self, before: usize) -> &Ending {
        self.endings.get(before).unwrap_or(&self.last)
    }
}

impl Need {
    fn of(input: &model::Input) -> Self {
        Self {
            key: leaked(&input.key),
            value_type: input.value_type,
            optional: input.optional,
        }
    }
}

/// Declares the input, with the Rust type of its type.
fn needing(needs: StepNeeds, input: &Need) -> StepNeeds {
    for_type(input.value_type, Needing { needs, input })
}

struct Needing<'n> {
    needs: StepNeeds,
    input: &'n Need,
}

impl ForType for Needing<'_> {
    type Output = StepNeeds;

    fn of<T: Storable>(self) -> StepNeeds {
        if self.input.optional {
            self.needs
                .optional_input(&OptionalInput::<T>::new(self.input.key))
        } else {
            self.needs.input(&Input::<T>::new(self.input.key))
        }
    }
}

impl Action {
    /// What the step does for the scripted action, if anything reaches beyond the step.
    fn of(action: &StepAction) -> Option<Result<Self, ModelError>> {
        match action {
            StepAction::Contribute { key, value }
            | StepAction::ContributeThenChange { key, value, .. } => Some(contribution(key, value)),
            StepAction::ChangeInput { .. } => None,
            StepAction::Emit {
                level,
                message,
                data,
            } => Some(emit(*level, message, data.as_ref())),
        }
    }
}

fn contribution(key: &str, value: &serde_json::Value) -> Result<Action, ModelError> {
    Ok(Action::Contribute(key.to_owned(), Typed::of(value)?))
}

fn emit(
    level: Level,
    message: &str,
    data: Option<&serde_json::Value>,
) -> Result<Action, ModelError> {
    Ok(Action::Emit {
        level,
        message: message.to_owned(),
        data: data.map(Typed::of).transpose()?,
    })
}

impl Ending {
    fn success() -> Self {
        Self {
            contributes: Vec::new(),
            end: End::Success,
        }
    }

    fn of(attempt: &model::Attempt) -> Result<Self, ModelError> {
        Ok(Self {
            contributes: attempt
                .contributes
                .iter()
                .map(typed_entry)
                .collect::<Result<_, _>>()?,
            end: End::of(&attempt.outcome)?,
        })
    }
}

fn typed_entry((key, value): &(String, serde_json::Value)) -> Result<(String, Typed), ModelError> {
    Ok((key.clone(), Typed::of(value)?))
}

impl End {
    fn of(outcome: &AttemptOutcome) -> Result<Self, ModelError> {
        Ok(match outcome {
            AttemptOutcome::Success => Self::Success,
            AttemptOutcome::Failure {
                reason,
                retriable: false,
            } => Self::Failure(reason_of(reason)?),
            AttemptOutcome::Failure {
                reason,
                retriable: true,
            } => Self::RetriableFailure(reason_of(reason)?),
            AttemptOutcome::Skipped(reason) => {
                Self::Skipped(reason.as_ref().map(reason_of).transpose()?)
            }
            AttemptOutcome::Error(message) => Self::Error(
                message
                    .clone()
                    .unwrap_or_else(|| "the scripted step fails".to_owned()),
            ),
        })
    }

    fn outcome(&self) -> Result<Outcome, Error> {
        match self {
            Self::Success => Ok(Outcome::success()),
            Self::Failure(reason) => Ok(Outcome::failure(reason.clone())),
            Self::RetriableFailure(reason) => Ok(Outcome::retriable_failure(reason.clone())),
            Self::Skipped(None) => Ok(Outcome::skipped()),
            Self::Skipped(Some(reason)) => Ok(Outcome::skipped_because(reason.clone())),
            Self::Error(message) => Err(Error::msg(message.clone())),
        }
    }
}

/// The reason, with its message and its details when the case gives them.
fn reason_of(reason: &model::Reason) -> Result<Reason, ModelError> {
    let coded = Reason::new(reason.code.clone());
    let explained = match &reason.message {
        Some(message) => coded.with_message(message.clone()),
        None => coded,
    };
    match &reason.details {
        Some(details) => Ok(Typed::of(details)?.given_to(Detailing { reason: explained })),
        None => Ok(explained),
    }
}

/// Gives a reason its details.
struct Detailing {
    reason: Reason,
}

impl Takes for Detailing {
    type Output = Reason;

    fn take<T: Storable>(self, details: T) -> Reason {
        self.reason.with_details(details)
    }
}

/// One attempt of a scripted step, with the handles its attempt gave it.
#[derive(derive_more::Debug)]
pub(crate) struct Attempting<'a, R> {
    script: &'a Script,
    ending: &'a Ending,
    contributor: Contributor<'a>,
    #[debug(skip)]
    reporter: R,
    runs: &'a StepsRun,
}

impl<R: Emits> Attempting<'_, R> {
    /// Does what the script says, then ends as the attempt's ending says.
    async fn perform(self) -> Result<Outcome, Error> {
        let Self {
            script,
            ending,
            mut contributor,
            mut reporter,
            runs,
        } = self;
        runs.one_more();
        for action in &script.actions {
            match action {
                Action::Contribute(key, value) => contribute(&mut contributor, key, value),
                Action::Emit {
                    level,
                    message,
                    data,
                } => {
                    let emitted = reporter.emit(*level, message.clone(), data.clone()).await;
                    if !script.ignores_failed_emits {
                        emitted?;
                    }
                }
            }
        }
        for (key, value) in &ending.contributes {
            contribute(&mut contributor, key, value);
        }
        ending.end.outcome()
    }
}

impl Step for Attempting<'_, StepReporter<'_>> {
    fn run(self) -> Result<Outcome, Error> {
        block_on(self.perform())
    }
}

impl AsyncStep for Attempting<'_, AsyncStepReporter<'_>> {
    fn run(self) -> impl Future<Output = Result<Outcome, Error>> + Send {
        self.perform()
    }
}

fn contribute(contributor: &mut Contributor<'_>, key: &str, value: &Typed) {
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

/// What emitting an event gives, once awaited.
pub(crate) type Emitted<'r> = Pin<Box<dyn Future<Output = Result<(), Interrupted>> + Send + 'r>>;

/// A step's reporter, of either mode, as the script uses it.
pub(crate) trait Emits: Send {
    fn emit(&mut self, level: Level, message: String, data: Option<Typed>) -> Emitted<'_>;
}

impl Emits for StepReporter<'_> {
    fn emit(&mut self, level: Level, message: String, data: Option<Typed>) -> Emitted<'_> {
        let emitted = match data {
            None => match level {
                Level::Info => self.info(message),
                Level::Warning => self.warning(message),
                Level::Error => self.error(message),
            },
            Some(data) => data.given_to(Reporting {
                reporter: self,
                level,
                message,
            }),
        };
        Box::pin(ready(emitted))
    }
}

/// Emits an event with data from a synchronous step.
struct Reporting<'r, 'a> {
    reporter: &'r mut StepReporter<'a>,
    level: Level,
    message: String,
}

impl Takes for Reporting<'_, '_> {
    type Output = Result<(), Interrupted>;

    fn take<T: Storable>(self, data: T) -> Result<(), Interrupted> {
        match self.level {
            Level::Info => self.reporter.info_with(self.message, data),
            Level::Warning => self.reporter.warning_with(self.message, data),
            Level::Error => self.reporter.error_with(self.message, data),
        }
    }
}

impl Emits for AsyncStepReporter<'_> {
    fn emit(&mut self, level: Level, message: String, data: Option<Typed>) -> Emitted<'_> {
        match data {
            None => match level {
                Level::Info => Box::pin(self.info(message)),
                Level::Warning => Box::pin(self.warning(message)),
                Level::Error => Box::pin(self.error(message)),
            },
            Some(data) => data.given_to(AsyncReporting {
                reporter: self,
                level,
                message,
            }),
        }
    }
}

/// Emits an event with data from an asynchronous step.
struct AsyncReporting<'r, 'a> {
    reporter: &'r mut AsyncStepReporter<'a>,
    level: Level,
    message: String,
}

impl<'r> Takes for AsyncReporting<'r, '_> {
    type Output = Emitted<'r>;

    fn take<T: Storable>(self, data: T) -> Emitted<'r> {
        match self.level {
            Level::Info => Box::pin(self.reporter.info_with(self.message, data)),
            Level::Warning => Box::pin(self.reporter.warning_with(self.message, data)),
            Level::Error => Box::pin(self.reporter.error_with(self.message, data)),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::journey::tests::{attempt, timed_out};

    fn declined() -> AttemptOutcome {
        AttemptOutcome::Failure {
            reason: model::Reason {
                code: "declined".to_owned(),
                message: None,
                details: None,
            },
            retriable: false,
        }
    }

    fn code(end: &End) -> Option<&str> {
        match end {
            End::Failure(reason) | End::RetriableFailure(reason) => Some(reason.code()),
            _ => None,
        }
    }

    #[rstest]
    #[case::the_first_row(0, "timeout")]
    #[case::the_last_row(1, "declined")]
    #[case::past_the_last_row(2, "declined")]
    fn an_attempt_ends_as_its_row_says_and_every_attempt_past_the_last_row_as_that_row_does(
        #[case] before: usize,
        #[case] expected: &str,
    ) {
        let step = model::Step {
            attempts: Some(vec![attempt(timed_out()), attempt(declined())]),
            ..model::Step::default()
        };
        let script = Script::of(&step).unwrap();
        assert_eq!(code(&script.ending(before).end), Some(expected));
    }

    #[test]
    fn a_step_whose_attempts_are_not_stated_succeeds() {
        let script = Script::of(&model::Step::default()).unwrap();
        assert!(matches!(script.ending(0).end, End::Success));
    }

    #[test]
    fn a_change_to_a_contributed_value_or_a_received_input_is_left_out_of_the_script() {
        let step = model::Step {
            actions: vec![
                StepAction::ContributeThenChange {
                    key: "receipt".to_owned(),
                    value: serde_json::json!("R-1"),
                    changed: serde_json::json!("R-2"),
                },
                StepAction::ChangeInput {
                    key: "items".to_owned(),
                    value: serde_json::json!(["nothing"]),
                },
            ],
            ..model::Step::default()
        };
        let script = Script::of(&step).unwrap();
        assert!(matches!(
            script.actions.as_slice(),
            [Action::Contribute(key, Typed::String(value))] if key == "receipt" && value == "R-1"
        ));
    }
}
