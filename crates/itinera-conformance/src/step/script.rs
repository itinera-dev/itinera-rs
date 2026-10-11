//! What a step's script says, read from the scenario model with its values typed.

use itinera::step::StepNeeds;

use super::ending::Ending;
use super::input::{needing, requested};
use crate::model::{self, Level, ModelError, StepAction};
use crate::value::{Data, Typed};

/// What a step's script says, with its values typed.
///
/// Rust moves a contributed value into the contribution, and gives a step its own copy of each
/// input, so a step that changes either afterwards changes nothing anyone else sees. The script
/// leaves those changes out.
#[derive(Debug)]
pub(super) struct Script {
    pub(super) inputs: Vec<Data>,
    /// What every attempt does before it ends, in order.
    pub(super) actions: Vec<Action>,
    /// How the attempts before the last stated one end, in order.
    endings: Vec<Ending>,
    /// How the last stated attempt ends, and every attempt after it.
    last: Ending,
    pub(super) construction_failure: Option<String>,
    pub(super) ignores_failed_emits: bool,
}

#[derive(Debug)]
pub(super) enum Action {
    Contribute(String, Typed),
    Emit {
        level: Level,
        message: String,
        data: Option<Typed>,
    },
}

impl Script {
    pub(super) fn of(step: &model::Step) -> Result<Self, ModelError> {
        let (endings, last) = match step.attempts.as_deref().and_then(<[_]>::split_last) {
            Some((last, endings)) => (
                endings.iter().map(Ending::of).collect::<Result<_, _>>()?,
                Ending::of(last)?,
            ),
            None => (Vec::new(), Ending::success()),
        };
        Ok(Self {
            inputs: step.inputs.iter().map(requested).collect(),
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
    pub(super) fn needs(&self) -> StepNeeds {
        self.inputs
            .iter()
            .fold(StepNeeds::new().contributor().reporter(), needing)
    }

    /// How the attempt that comes after `before` others ends.
    pub(super) fn ending(&self, before: usize) -> &Ending {
        self.endings.get(before).unwrap_or(&self.last)
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::journey::fixtures::{attempt, timed_out};
    use crate::model::AttemptOutcome;
    use crate::step::ending::End;

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
