//! An input adapter's read access to the data bag.

use super::DataBag;
use crate::value::{AnyValue, Value};

/// Read access to the data bag, which an input adapter declares and takes for one call. It
/// shows everything committed before the attempt being built, and lives no longer than that
/// call, so an adapter cannot keep it for a later one. It offers no way to write.
///
/// A read is the adapter's own business: it emits no event and aborts nothing, whatever it
/// finds.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::journey::Read;
/// use itinera::policy::{InputAdapter, Requested};
/// use itinera::workflow::Supplied;
///
/// struct Orders;
///
/// impl Orders {
///     fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Supplied> {
///         let key = format!("price.{}", got.step_name());
///         match got.data_bag()?.read::<i64>(&key) {
///             Read::Present(price) => Ok(Supplied::value(price)),
///             Read::Absent | Read::OtherType => Ok(Supplied::FromDataBag),
///         }
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct DataBagAccess<'a> {
    data_bag: &'a DataBag,
}

impl<'a> DataBagAccess<'a> {
    pub(crate) fn new(data_bag: &'a DataBag) -> Self {
        Self { data_bag }
    }

    /// Reads the value under a key as a `T`: the adapter's own copy of it, or what kept it from
    /// being one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{DataBagAccess, Read};
    ///
    /// fn quantity(access: DataBagAccess<'_>) -> i64 {
    ///     match access.read("quantity") {
    ///         Read::Present(quantity) => quantity,
    ///         Read::Absent | Read::OtherType => 1,
    ///     }
    /// }
    /// ```
    pub fn read<T: Value>(&self, key: &str) -> Read<T> {
        self.data_bag.read(key)
    }
}

/// What a read through a [`DataBagAccess`] found under a key.
///
/// # Examples
///
/// ```
/// use itinera::journey::Read;
///
/// fn or_zero(read: Read<i64>) -> i64 {
///     match read {
///         Read::Present(value) => value,
///         Read::Absent | Read::OtherType => 0,
///     }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Read<T> {
    /// A value of the type asked for.
    Present(T),
    /// The data bag holds nothing under the key.
    Absent,
    /// The data bag holds a value of another type under the key.
    OtherType,
}

impl<T> Read<T> {
    /// The value, when one of the type asked for was found; `None` when the key holds nothing or a
    /// value of another type.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::Read;
    ///
    /// fn quantity(read: Read<i64>) -> i64 {
    ///     read.present().unwrap_or(1)
    /// }
    ///
    /// assert_eq!(quantity(Read::Present(3)), 3);
    /// assert_eq!(quantity(Read::OtherType), 1);
    /// ```
    pub fn present(self) -> Option<T> {
        match self {
            Self::Present(value) => Some(value),
            Self::Absent | Self::OtherType => None,
        }
    }
}

impl<T: Value> Read<T> {
    /// What reading a value as a `T` finds, given the value under the key, if any.
    pub(crate) fn of(value: Option<&AnyValue>) -> Self {
        value.map_or(Self::Absent, as_read)
    }
}

fn as_read<T: Value>(value: &AnyValue) -> Read<T> {
    value
        .downcast_ref::<T>()
        .cloned()
        .map_or(Read::OtherType, Read::Present)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::a_value_of_the_type_asked_for(Some(AnyValue::new(42_i64)), Read::Present(42))]
    #[case::a_key_that_holds_nothing(None, Read::Absent)]
    #[case::a_value_of_another_type(Some(AnyValue::new("ada".to_string())), Read::OtherType)]
    fn a_read_reports_what_the_key_holds(
        #[case] value: Option<AnyValue>,
        #[case] expected: Read<i64>,
    ) {
        assert_eq!(Read::of(value.as_ref()), expected);
    }

    #[rstest]
    #[case::a_value_of_the_type_asked_for(Read::Present(42), Some(42))]
    #[case::a_key_that_holds_nothing(Read::Absent, None)]
    #[case::a_value_of_another_type(Read::OtherType, None)]
    fn only_a_value_of_the_type_asked_for_is_present(
        #[case] read: Read<i64>,
        #[case] present: Option<i64>,
    ) {
        assert_eq!(read.present(), present);
    }
}
