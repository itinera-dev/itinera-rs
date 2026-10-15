//! The scenario's input adapters, declared with their scripted answers.

use itinera::error::Error;
use itinera::journey::{DataBag, Read};
use itinera::policy::{HookNeeds, InputAdapter, Requested};
use itinera::step::StepName;
use itinera::value::Value as Storable;
use itinera::workflow::{InputAdapterDescriptor, Supplied};

use super::{ScriptedWorkflow, leaked, step_name};
use crate::model::{Adapter, Answer, ModelError, ValueType};
use crate::policy::requiring_from_workflow;
use crate::value::{ForType, Typed, for_type};

pub(super) fn input_adapter(
    adapter: &Adapter,
) -> Result<InputAdapterDescriptor<ScriptedWorkflow>, ModelError> {
    let steps = adapter
        .steps
        .iter()
        .map(String::as_str)
        .map(step_name)
        .collect::<Result<Vec<_>, _>>()?;
    let (first, rest) = steps
        .split_first()
        .ok_or_else(|| ModelError::AdapterWithoutSteps(adapter.name.clone()))?;
    let answers = adapter
        .answers
        .iter()
        .map(Answering::of)
        .collect::<Result<Vec<_>, _>>()?;
    let needs = adapter
        .requests
        .iter()
        .fold(HookNeeds::new(), requiring_from_workflow);
    let needs = if adapter.data_bag {
        needs.data_bag()
    } else {
        needs
    };
    Ok(rest.iter().copied().fold(
        InputAdapterDescriptor::new(leaked(&adapter.name), *first, move |_, mut got| {
            answer(&answers, &mut got)
        })
        .needing(needs),
        InputAdapterDescriptor::step,
    ))
}

/// A scripted adapter's answer for one key, with its value typed.
#[derive(Debug)]
struct Answering {
    key: String,
    answer: Answered,
}

#[derive(Debug)]
enum Answered {
    Value(Typed),
    Nothing,
    Fails(String),
    ReadFromDataBag(String, ValueType),
    StepName,
}

impl Answering {
    fn of((key, answer): &(String, Answer)) -> Result<Self, ModelError> {
        let answer = match answer {
            Answer::Value(value) => Answered::Value(Typed::of(value)?),
            Answer::Nothing => Answered::Nothing,
            Answer::Fails(message) => Answered::Fails(message.clone()),
            Answer::ReadFromDataBag(key, value_type) => {
                Answered::ReadFromDataBag(key.clone(), *value_type)
            }
            Answer::StepName => Answered::StepName,
        };
        Ok(Self {
            key: key.clone(),
            answer,
        })
    }

    fn is_for(&self, key: &str) -> bool {
        self.key == key
    }
}

/// What the adapter answers for the key it is called for: nothing for a key its script does not
/// mention.
fn answer(
    answers: &[Answering],
    got: &mut Requested<'_, ScriptedWorkflow, InputAdapter>,
) -> Result<Supplied, Error> {
    let key = got.key();
    let step = got.step_name();
    answered(answers, key, step, || got.data_bag())
}

/// What the adapter answers when a test calls it alone for the input `key` of the step, with
/// the read access a data bag of its own lends.
pub(crate) fn answer_alone(
    adapter: &Adapter,
    key: &str,
    step: &str,
    bag: &DataBag,
) -> Result<Result<Supplied, Error>, ModelError> {
    let answers = adapter
        .answers
        .iter()
        .map(Answering::of)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(answered(&answers, key, step_name(step)?, || Ok(bag)))
}

/// What the adapter answers for the input `key` of the step, reading the data bag through the
/// access `access` gives, when its answer reads it.
fn answered<'a>(
    answers: &[Answering],
    key: &str,
    step: StepName,
    access: impl FnOnce() -> Result<&'a DataBag, Error>,
) -> Result<Supplied, Error> {
    let answered = answers
        .iter()
        .find(|answering| answering.is_for(key))
        .map(|answering| &answering.answer);
    match answered {
        Some(Answered::Value(value)) => Ok(Supplied::Value(value.clone().erased())),
        Some(Answered::Fails(message)) => Err(Error::msg(message.clone())),
        Some(Answered::ReadFromDataBag(key, value_type)) => Ok(for_type(
            *value_type,
            Reading {
                access: access()?,
                key,
            },
        )),
        Some(Answered::StepName) => Ok(Supplied::value(step.to_string())),
        Some(Answered::Nothing) | None => Ok(Supplied::FromDataBag),
    }
}

/// Reads a key through an adapter's access to the data bag, as one type.
struct Reading<'a, 'k> {
    access: &'a DataBag,
    key: &'k str,
}

impl ForType for Reading<'_, '_> {
    type Output = Supplied;

    fn of<T: Storable>(self) -> Supplied {
        match self.access.read::<T>(self.key) {
            Read::Present(value) => Supplied::value(value),
            Read::Absent | Read::OtherType => Supplied::FromDataBag,
        }
    }
}
