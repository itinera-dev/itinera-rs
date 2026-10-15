//! The trait that tells a required input from an optional one.

use super::{Input, OptionalInput};
use crate::value::{AnyValue, Value};

mod sealed;

/// A token for data requested by key: an [`Input`], whose value is required, or an
/// [`OptionalInput`], whose value may be absent.
///
/// Steps, rescue steps and hooks declare their data with the same method for both, and take it
/// back with the same method: the token says whether the value is required, and so what taking it
/// gives. Only itinera implements it.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::step::{Input, OptionalInput, Resolved, StepNeeds};
///
/// const AMOUNT: Input<i64> = Input::new("amount");
/// const DISCOUNT: OptionalInput<i64> = OptionalInput::new("discount");
///
/// fn needs() -> StepNeeds {
///     StepNeeds::new().input(&AMOUNT).input(&DISCOUNT)
/// }
///
/// fn total(got: &mut Resolved<'_>) -> Result<i64> {
///     let amount: i64 = got.input(&AMOUNT)?;
///     let discount: Option<i64> = got.input(&DISCOUNT)?;
///     Ok(amount - discount.unwrap_or(0))
/// }
/// ```
pub trait InputToken: sealed::Sealed {
    /// The type of the value under the key.
    type Value: Value;
    /// What taking the data gives: the value for an [`Input`], and an `Option` of it for an
    /// [`OptionalInput`], `None` when the key is absent.
    type Given;
}

impl<T: Value> InputToken for Input<T> {
    type Value = T;
    type Given = T;
}

impl<T: Value> sealed::Sealed for Input<T> {
    const OPTIONAL: bool = false;

    fn key(&self) -> &'static str {
        self.key
    }

    fn given(slot: Option<Option<AnyValue>>) -> Option<<Self as InputToken>::Given> {
        slot.flatten().and_then(AnyValue::downcast)
    }
}

impl<T: Value> InputToken for OptionalInput<T> {
    type Value = T;
    type Given = Option<T>;
}

impl<T: Value> sealed::Sealed for OptionalInput<T> {
    const OPTIONAL: bool = true;

    fn key(&self) -> &'static str {
        self.key
    }

    fn given(slot: Option<Option<AnyValue>>) -> Option<<Self as InputToken>::Given> {
        slot.and_then(present_or_absent)
    }
}

/// A value that may be absent, as an optional input gives it, or `None` when it has another type.
fn present_or_absent<T: Value>(value: Option<AnyValue>) -> Option<Option<T>> {
    match value {
        Some(value) => value.downcast().map(Some),
        None => Some(None),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::sealed::Sealed;
    use super::*;

    fn present<T: Value>(value: T) -> Option<Option<AnyValue>> {
        Some(Some(AnyValue::new(value)))
    }

    #[rstest]
    #[case::a_value_of_its_type(present(3_i64), Some(3))]
    #[case::a_value_of_another_type(present("three".to_owned()), None)]
    #[case::an_absent_value(Some(None), None)]
    #[case::no_value_declared_or_left(None, None)]
    fn an_input_gives_only_a_present_value_of_its_type(
        #[case] slot: Option<Option<AnyValue>>,
        #[case] given: Option<i64>,
    ) {
        assert_eq!(<Input<i64> as Sealed>::given(slot), given);
    }

    #[rstest]
    #[case::a_value_of_its_type(present(3_i64), Some(Some(3)))]
    #[case::a_value_of_another_type(present("three".to_owned()), None)]
    #[case::an_absent_value(Some(None), Some(None))]
    #[case::no_value_declared_or_left(None, None)]
    fn an_optional_input_gives_a_value_of_its_type_or_its_absence(
        #[case] slot: Option<Option<AnyValue>>,
        #[case] given: Option<Option<i64>>,
    ) {
        assert_eq!(<OptionalInput<i64> as Sealed>::given(slot), given);
    }
}
