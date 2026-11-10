//! What scripted input adapters do.

use serde_json::Value;

use super::value::ValueType;

/// An input adapter the workflow declares, and the steps it is attached to.
#[derive(Debug)]
pub(crate) struct Adapter {
    pub(crate) name: String,
    pub(crate) steps: Vec<String>,
    /// Its answers, in the order written; it returns nothing for any key none of them is for.
    pub(crate) answers: Vec<Answering>,
    /// Data from the workflow it requests, all required.
    pub(crate) requests: Vec<(String, ValueType)>,
    /// Whether it requests read access to the data bag.
    pub(crate) data_bag: bool,
}

impl Adapter {
    /// An adapter of this name, attached to these steps, that answers nothing yet.
    pub(crate) fn named(name: String, steps: Vec<String>) -> Self {
        Self {
            name,
            steps,
            answers: Vec::new(),
            requests: Vec::new(),
            data_bag: false,
        }
    }

    pub(crate) fn is_named(&self, name: &str) -> bool {
        self.name == name
    }
}

/// What an adapter answers for a key: for every step, or only when called for one step.
#[derive(Debug, PartialEq)]
pub(crate) struct Answering {
    pub(crate) key: String,
    pub(crate) step: Option<String>,
    pub(crate) answer: Answer,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Answer {
    Value(Value),
    /// The input is read from the data bag.
    Nothing,
    Fails(String),
    /// The value under this key of the data bag, read through its access, when it has this type;
    /// nothing otherwise.
    ReadFromDataBag(String, ValueType),
    /// The name of the step being built, as a string.
    StepName,
}

/// Whether an answer kept to this step, or to none, is for the step.
pub(crate) fn is_for_step(only: Option<&str>, step: &str) -> bool {
    only.is_none() || only == Some(step)
}

/// The value an adapter supplies for this key, when its answer is for it and is a value.
pub(crate) fn answer_for<'a>(answering: &'a Answering, key: &str) -> Option<&'a Value> {
    if answering.key == key {
        answering.answer.value()
    } else {
        None
    }
}

impl Answer {
    /// The value the adapter supplies, if it supplies one.
    pub(crate) fn value(&self) -> Option<&Value> {
        match self {
            Self::Value(value) => Some(value),
            Self::Nothing | Self::Fails(_) | Self::ReadFromDataBag(..) | Self::StepName => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::for_every_step(None, "ship", true)]
    #[case::for_this_step(Some("charge"), "charge", true)]
    #[case::for_another_step(Some("charge"), "ship", false)]
    fn an_answer_kept_to_a_step_is_the_answer_for_that_step_alone(
        #[case] only: Option<&str>,
        #[case] step: &str,
        #[case] is_for: bool,
    ) {
        assert_eq!(is_for_step(only, step), is_for);
    }
}
