//! A step's inputs: declaring them, and taking what the step was built with.

use itinera::error::Error;
use itinera::step::{Input, OptionalInput, Resolved, StepNeeds};
use itinera::value::Value as Storable;
use serde_json::Value;

use crate::model;
use crate::value::{Data, ForType, for_type, json};

/// What the step requests for the input.
pub(super) fn requested(input: &model::Input) -> Data {
    Data::of(&input.key, input.value_type, input.optional)
}

/// The input's key and its value as JSON, or `None` when it was absent.
pub(super) fn taken<M>(
    got: &mut Resolved<'_, M>,
    input: &Data,
) -> Result<(String, Option<Value>), Error> {
    let value = for_type(input.value_type, Taking { got, input })?;
    Ok((input.key.to_owned(), value))
}

struct Taking<'g, 'a, 'n, M> {
    got: &'g mut Resolved<'a, M>,
    input: &'n Data,
}

impl<M> ForType for Taking<'_, '_, '_, M> {
    type Output = Result<Option<Value>, Error>;

    fn of<T: Storable>(self) -> Self::Output {
        if self.input.optional {
            self.got
                .optional_input(&OptionalInput::<T>::new(self.input.key))?
                .map(json)
                .transpose()
        } else {
            json(self.got.input(&Input::<T>::new(self.input.key))?).map(Some)
        }
    }
}

/// Declares the input, with the Rust type of its type.
pub(super) fn needing(needs: StepNeeds, input: &Data) -> StepNeeds {
    for_type(input.value_type, Needing { needs, input })
}

struct Needing<'n> {
    needs: StepNeeds,
    input: &'n Data,
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
