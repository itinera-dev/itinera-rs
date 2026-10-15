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
    /// assert!(matches!(supplied, Supplied::Value(value) if value.downcast_ref::<i64>() == Some(&40)));
    /// ```
    pub fn value<T: Value>(value: T) -> Self {
        Self::Value(AnyValue::new(value))
    }
}
