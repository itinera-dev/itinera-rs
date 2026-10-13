//! The data bag: the data of one journey.

use std::collections::BTreeMap;

use super::{OutputError, Read};
use crate::value::{AnyValue, Value};

/// The data of one journey: a map from string keys to values.
///
/// It starts with the instance's initial data. At the end of a journey that succeeded or failed,
/// the result carries it as the journey's output. Its keys are listed in order. With the `testing`
/// feature it implements `Default`, so that a test builds one with `with`.
///
/// # Examples
///
/// ```
/// use itinera::journey::DataBag;
/// use itinera::value::AnyValue;
///
/// fn amount(data: &DataBag) -> Option<i64> {
///     data.get("amount").and_then(as_amount)
/// }
///
/// fn as_amount(value: &AnyValue) -> Option<i64> {
///     value.downcast_ref().copied()
/// }
///
/// fn listed(data: &DataBag) -> Vec<&str> {
///     data.keys().collect()
/// }
/// ```
#[derive(Clone, Debug, derive_more::IntoIterator)]
#[cfg_attr(any(test, feature = "testing"), derive(Default))]
#[into_iterator(owned, ref)]
pub struct DataBag {
    values: BTreeMap<String, AnyValue>,
}

impl DataBag {
    pub(crate) fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    /// Puts a value under a key, and returns the value it replaced, if any.
    pub(crate) fn insert(&mut self, key: String, value: AnyValue) -> Option<AnyValue> {
        self.values.insert(key, value)
    }

    /// The data bag with a value put under a key, replacing what it held there, for a test to
    /// build the data bag an input adapter reads. It is available with the `testing` feature.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::DataBag;
    ///
    /// let data = DataBag::default().with("quantity", 3_i64).with("coupon", "SPRING".to_string());
    /// assert_eq!(data.keys().collect::<Vec<_>>(), ["coupon", "quantity"]);
    /// ```
    #[cfg(any(test, feature = "testing"))]
    #[must_use]
    pub fn with<T: Value>(mut self, key: impl Into<String>, value: T) -> Self {
        self.insert(key.into(), AnyValue::new(value));
        self
    }

    /// Lends read access to the data bag, for a test to pass to an input adapter. It is
    /// available with the `testing` feature.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{DataBag, Read};
    ///
    /// let data = DataBag::default().with("quantity", 3_i64);
    /// assert_eq!(data.access().read::<i64>("quantity"), Read::Present(3));
    /// ```
    #[cfg(any(test, feature = "testing"))]
    #[must_use]
    pub fn access(&self) -> super::DataBagAccess<'_> {
        super::DataBagAccess::new(self)
    }

    /// The value under a key, if there is one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::DataBag;
    ///
    /// fn customer(data: &DataBag) -> Option<&str> {
    ///     data.get("customer")?.downcast_ref::<String>().map(String::as_str)
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<&AnyValue> {
        self.values.get(key)
    }

    /// The keys, in order.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::DataBag;
    ///
    /// fn has_amount(data: &DataBag) -> bool {
    ///     data.keys().any(is_amount)
    /// }
    ///
    /// fn is_amount(key: &str) -> bool {
    ///     key == "amount"
    /// }
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }

    /// What the data bag holds under a key, read as a value of this type.
    pub(crate) fn read<T: Value>(&self, key: &str) -> Read<T> {
        Read::of(self.get(key))
    }

    /// The value under a key, which an output requires.
    ///
    /// # Errors
    ///
    /// When the data bag holds no value under the key, or a value of another type.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{DataBag, OutputError};
    ///
    /// fn receipt(data: &DataBag) -> Result<String, OutputError> {
    ///     data.required("receipt")
    /// }
    /// ```
    pub fn required<T: Value>(&self, key: &str) -> Result<T, OutputError> {
        match self.read(key) {
            Read::Present(value) => Ok(value),
            Read::Absent => Err(OutputError::Missing {
                key: key.to_owned(),
            }),
            Read::OtherType => Err(wrong_type::<T>(key)),
        }
    }

    /// The value under a key, or `None` when the data bag holds none.
    ///
    /// # Errors
    ///
    /// When the data bag holds a value of another type under the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{DataBag, OutputError};
    ///
    /// fn note(data: &DataBag) -> Result<Option<String>, OutputError> {
    ///     data.optional("note")
    /// }
    /// ```
    pub fn optional<T: Value>(&self, key: &str) -> Result<Option<T>, OutputError> {
        match self.read(key) {
            Read::Present(value) => Ok(Some(value)),
            Read::Absent => Ok(None),
            Read::OtherType => Err(wrong_type::<T>(key)),
        }
    }
}

fn wrong_type<T>(key: &str) -> OutputError {
    OutputError::WrongType {
        key: key.to_owned(),
        expected: std::any::type_name::<T>(),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::journey::fixtures::bag;

    fn amount(value: &AnyValue) -> Option<i64> {
        value.downcast_ref().copied()
    }

    #[test]
    fn a_data_bag_lists_its_keys_in_order() {
        assert_eq!(bag().keys().collect::<Vec<_>>(), ["amount", "customer"]);
    }

    #[test]
    fn a_data_bag_gives_the_value_under_a_key() {
        let data = bag();
        assert_eq!(data.get("amount").and_then(amount), Some(42));
        assert!(data.get("missing").is_none());
    }

    #[test]
    fn a_data_bag_iterates_over_its_keys_and_values() {
        let keys: Vec<String> = bag().into_iter().map(|(key, _)| key).collect();
        assert_eq!(keys, ["amount", "customer"]);
    }

    #[rstest]
    #[case::a_value_of_the_type_asked_for("amount", Ok(42))]
    #[case::a_key_the_data_bag_does_not_hold("discount", Err(OutputError::Missing { key: "discount".to_owned() }))]
    #[case::a_value_of_another_type(
        "customer",
        Err(OutputError::WrongType { key: "customer".to_owned(), expected: "i64" })
    )]
    fn a_required_value_is_present_and_of_its_type(
        #[case] key: &str,
        #[case] expected: Result<i64, OutputError>,
    ) {
        assert_eq!(bag().required::<i64>(key), expected);
    }

    #[rstest]
    #[case::a_value_of_the_type_asked_for("amount", Ok(Some(42)))]
    #[case::a_key_the_data_bag_does_not_hold("discount", Ok(None))]
    #[case::a_value_of_another_type(
        "customer",
        Err(OutputError::WrongType { key: "customer".to_owned(), expected: "i64" })
    )]
    fn an_optional_value_is_absent_or_of_its_type(
        #[case] key: &str,
        #[case] expected: Result<Option<i64>, OutputError>,
    ) {
        assert_eq!(bag().optional::<i64>(key), expected);
    }
}
