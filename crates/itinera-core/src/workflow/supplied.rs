//! What an input adapter returns for the input it was asked for.

use crate::value::{AnyValue, Value};

/// What an input adapter returns for the input it was asked for: a value it supplies, or nothing,
/// so that the input is read from the data bag.
///
/// An input adapter declared with `#[input_adapter]` returns it with the shorthands
/// `Self::supply(..)` and `Self::from_data_bag()`.
///
/// # Examples
///
/// ```
/// use itinera::workflow::Supplied;
///
/// fn pricing(key: &str, price: i64) -> Supplied {
///     if key == "amount" {
///         Supplied::value(price)
///     } else {
///         Supplied::FromDataBag
///     }
/// }
///
/// assert!(matches!(pricing("amount", 40), Supplied::Value(_)));
/// assert!(matches!(pricing("currency", 40), Supplied::FromDataBag));
/// ```
#[derive(Debug)]
pub enum Supplied {
    /// The input's value, which must have the type the step declares.
    Value(AnyValue),
    /// Nothing: the input is read from the data bag.
    FromDataBag,
}

impl Supplied {
    /// Supplies this value for the input.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::Supplied;
    ///
    /// let supplied = Supplied::value(40_i64);
    /// assert_eq!(supplied.downcast_ref::<i64>(), Some(&40));
    /// ```
    pub fn value<T: Value>(value: T) -> Self {
        Self::Value(AnyValue::new(value))
    }

    /// The value supplied, if there is one and it has type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::workflow::Supplied;
    ///
    /// let supplied = Supplied::value(40_i64);
    /// assert_eq!(supplied.downcast_ref::<i64>(), Some(&40));
    /// assert_eq!(supplied.downcast_ref::<String>(), None);
    /// assert_eq!(Supplied::FromDataBag.downcast_ref::<i64>(), None);
    /// ```
    pub fn downcast_ref<T: Value>(&self) -> Option<&T> {
        match self {
            Self::Value(value) => value.downcast_ref(),
            Self::FromDataBag => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::a_value_of_that_type(Supplied::value(40_i64), Some(&40))]
    #[case::a_value_of_another_type(Supplied::value("forty".to_owned()), None)]
    #[case::nothing(Supplied::FromDataBag, None)]
    fn only_a_supplied_value_of_the_type_asked_for_is_read(
        #[case] supplied: Supplied,
        #[case] read: Option<&i64>,
    ) {
        assert_eq!(supplied.downcast_ref::<i64>(), read);
    }
}
