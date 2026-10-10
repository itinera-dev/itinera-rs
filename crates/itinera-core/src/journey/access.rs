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
/// use itinera::error::Error;
/// use itinera::journey::Read;
/// use itinera::policy::{InputAdapter, Requested};
/// use itinera::value::AnyValue;
///
/// struct Orders;
///
/// impl Orders {
///     fn pricing(&self, mut got: Requested<'_, Self, InputAdapter>) -> Result<Option<AnyValue>, Error> {
///         let key = format!("price.{}", got.step_name());
///         match got.data_bag()?.read::<i64>(&key) {
///             Read::Present(price) => Ok(Some(AnyValue::new(price))),
///             Read::Absent | Read::OtherType => Ok(None),
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
        self.data_bag.get(key).map_or(Read::Absent, as_read)
    }
}

fn as_read<T: Value>(value: &AnyValue) -> Read<T> {
    value
        .downcast_ref::<T>()
        .cloned()
        .map_or(Read::OtherType, Read::Present)
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn priced() -> DataBag {
        let mut data_bag = DataBag::new();
        data_bag.insert("price".to_owned(), AnyValue::new(7_i64));
        data_bag
    }

    #[rstest]
    #[case::a_value_of_the_type_asked_for("price", Read::Present(7))]
    #[case::a_key_the_data_bag_does_not_hold("discount", Read::Absent)]
    fn a_read_reports_what_the_data_bag_holds(#[case] key: &str, #[case] expected: Read<i64>) {
        assert_eq!(DataBagAccess::new(&priced()).read::<i64>(key), expected);
    }

    #[test]
    fn a_read_as_another_type_reports_it() {
        assert_eq!(
            DataBagAccess::new(&priced()).read::<String>("price"),
            Read::OtherType
        );
    }
}
