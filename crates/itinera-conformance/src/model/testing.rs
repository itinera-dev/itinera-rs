//! What a scenario of test support declares: the language's recording reporter, the fakes that
//! replace service providers and the script of a scripted run.

use serde_json::Value;

use super::table::Row;
use super::value::entries;
use super::{Implementation, Level, ModelError};

/// What the scenario asks of the language's test support before it acts.
#[derive(Debug, Default)]
pub(crate) struct Testing {
    /// Whether the instance has the recording reporter of test support.
    pub(crate) recording_reporter: bool,
    /// The fakes that replace providers, in the order written.
    pub(crate) fakes: Vec<Fake>,
    /// The rows of the script, in the order written.
    pub(crate) script: Vec<ScriptRow>,
    /// Whether the script ensures every step is scripted.
    pub(crate) every_step_scripted: bool,
}

/// A fake that replaces the provider of this name.
#[derive(Debug)]
pub(crate) struct Fake {
    pub(crate) provider: String,
    pub(crate) makes: Faking,
}

/// What a fake does on each call.
#[derive(Debug)]
pub(crate) enum Faking {
    /// Makes an instance of this implementation of `labeller`, with this data, if any.
    Labellers {
        implementation: Implementation,
        data: Option<Value>,
    },
    /// Fails with this message.
    Fails(String),
    /// Makes a `meter`, another service than the providers of the cases' `labeller` make.
    Meters,
}

/// One row of the script: what the attempts it chooses of one step do.
#[derive(Debug, PartialEq)]
pub(crate) struct ScriptRow {
    pub(crate) step: String,
    pub(crate) attempts: Chosen,
    pub(crate) plays: Played,
}

/// What the attempts a row chooses do: run the real step, or play a script.
#[derive(Debug, PartialEq)]
pub(crate) enum Played {
    ForReal,
    Scripted { acts: Acts, ending: ScriptedEnding },
}

/// What scripted attempts do before they end.
#[derive(Debug, PartialEq)]
pub(crate) struct Acts {
    /// The contributions, in the order written.
    pub(crate) contributes: Vec<(String, Value)>,
    pub(crate) emits: Option<(Level, String)>,
    /// The entries of global metadata set, in the order written.
    pub(crate) sets: Vec<(String, Value)>,
}

/// The attempts a row chooses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Chosen {
    /// The attempt of this number, as written, which the language's test support checks.
    Number(u32),
    Every,
    Remaining,
}

/// How the attempts a row chooses end.
#[derive(Debug, PartialEq)]
pub(crate) enum ScriptedEnding {
    Success,
    /// A failure with this code.
    Failure(String),
    /// A skip, with this code when it has a reason.
    Skipped(Option<String>),
    /// An abnormal termination whose error has this message.
    AbnormalTermination(String),
}

impl ScriptRow {
    pub(crate) fn of(row: &Row) -> Result<Self, ModelError> {
        Ok(Self {
            step: row.required("step")?.to_owned(),
            attempts: chosen(row.required("attempt")?)?,
            plays: played(row)?,
        })
    }
}

impl Acts {
    /// Whether the attempts contribute, emit and set global metadata nothing.
    fn are_none(&self) -> bool {
        self.contributes.is_empty() && self.emits.is_none() && self.sets.is_empty()
    }
}

/// What a row's attempts do; a row that runs the real step does nothing else.
fn played(row: &Row) -> Result<Played, ModelError> {
    let acts = Acts {
        contributes: listed(row.optional("contributes"))?,
        emits: row
            .optional("emits")
            .map(|kind| emit(row, kind))
            .transpose()?,
        sets: listed(row.optional("sets global metadata"))?,
    };
    match row.required("ending")? {
        "real run" if acts.are_none() => Ok(Played::ForReal),
        "real run" => Err(ModelError::Cell(
            "real run",
            "a row that also acts".to_owned(),
        )),
        ending => Ok(Played::Scripted {
            acts,
            ending: ending_of(row, ending)?,
        }),
    }
}

fn chosen(cell: &str) -> Result<Chosen, ModelError> {
    match cell {
        "every" => Ok(Chosen::Every),
        "remaining" => Ok(Chosen::Remaining),
        number => number
            .parse()
            .map(Chosen::Number)
            .map_err(|_| ModelError::Cell("attempt", number.to_owned())),
    }
}

/// The entries of a JSON object written in a cell, in the order written, or none.
fn listed(cell: Option<&str>) -> Result<Vec<(String, Value)>, ModelError> {
    Ok(cell.map(entries).transpose()?.unwrap_or_default())
}

fn emit(row: &Row, kind: &str) -> Result<(Level, String), ModelError> {
    Ok((
        Level::of("step", kind)?,
        row.required("emit message")?.to_owned(),
    ))
}

fn ending_of(row: &Row, ending: &str) -> Result<ScriptedEnding, ModelError> {
    Ok(match ending {
        "success" => ScriptedEnding::Success,
        "failure" => ScriptedEnding::Failure(row.required("code")?.to_owned()),
        "skipped" => ScriptedEnding::Skipped(row.optional("code").map(str::to_owned)),
        "abnormal termination" => {
            ScriptedEnding::AbnormalTermination(row.required("message")?.to_owned())
        }
        other => return Err(ModelError::Cell("ending", other.to_owned())),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn row(cells: &[(&str, &str)]) -> Row {
        cells
            .iter()
            .map(|(column, cell)| ((*column).to_owned(), (*cell).to_owned()))
            .collect()
    }

    #[test]
    fn a_row_keeps_what_it_contributes_and_sets_in_the_order_written() {
        let script = ScriptRow::of(&row(&[
            ("step", "charge"),
            ("attempt", "remaining"),
            ("contributes", r#"{"b": 1, "a": 2}"#),
            ("emits", "step_warning"),
            ("emit message", "low stock"),
            ("sets global metadata", r#"{"user_id": "u-7"}"#),
            ("ending", "skipped"),
        ]))
        .unwrap();
        assert_eq!(
            script,
            ScriptRow {
                step: "charge".to_owned(),
                attempts: Chosen::Remaining,
                plays: Played::Scripted {
                    acts: Acts {
                        contributes: vec![("b".to_owned(), json!(1)), ("a".to_owned(), json!(2))],
                        emits: Some((Level::Warning, "low stock".to_owned())),
                        sets: vec![("user_id".to_owned(), json!("u-7"))],
                    },
                    ending: ScriptedEnding::Skipped(None),
                },
            }
        );
    }

    #[test]
    fn a_row_that_runs_the_real_step_does_nothing_else() {
        let acting = row(&[
            ("step", "charge"),
            ("attempt", "2"),
            ("contributes", r#"{"a": 1}"#),
            ("ending", "real run"),
        ]);
        assert!(ScriptRow::of(&acting).is_err());
    }
}
