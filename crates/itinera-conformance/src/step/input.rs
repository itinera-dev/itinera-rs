//! A step's inputs: declaring them, and taking what the step was built with.

use itinera::error::Error;
use itinera::step::{Input, OptionalInput};
use itinera::value::Value as Storable;
use serde_json::Value;

use super::building::{Builds, Needs};
use crate::model;
use crate::value::{Data, ForType, for_type, json};

/// What the step requests for the input.
pub(super) fn requested(input: &model::Input) -> Data {
    Data::of(&input.key, input.value_type, input.optional)
}

/// The input's key and its value as JSON, or `None` when it was absent.
pub(super) fn taken<'a>(
    got: &mut impl Builds<'a>,
    input: &Data,
) -> Result<(String, Option<Value>), Error> {
    let value = for_type(input.value_type, Taking { got, input })?;
    Ok((input.key.to_owned(), value))
}

struct Taking<'g, 'n, G> {
    got: &'g mut G,
    input: &'n Data,
}

impl<'a, G: Builds<'a>> ForType for Taking<'_, '_, G> {
    type Output = Result<Option<Value>, Error>;

    fn of<T: Storable>(self) -> Self::Output {
        if self.input.optional {
            self.got
                .input(&OptionalInput::<T>::new(self.input.key))?
                .map(json)
                .transpose()
        } else {
            json(self.got.input(&Input::<T>::new(self.input.key))?).map(Some)
        }
    }
}

/// Declares the input, with the Rust type of its type.
pub(super) fn needing<N: Needs>(needs: N, input: &Data) -> N {
    for_type(input.value_type, Needing { needs, input })
}

struct Needing<'n, N> {
    needs: N,
    input: &'n Data,
}

impl<N: Needs> ForType for Needing<'_, N> {
    type Output = N;

    fn of<T: Storable>(self) -> N {
        if self.input.optional {
            self.needs.input(&OptionalInput::<T>::new(self.input.key))
        } else {
            self.needs.input(&Input::<T>::new(self.input.key))
        }
    }
}
