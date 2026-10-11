//! The data bag: the data of one journey.

use std::collections::BTreeMap;

use crate::value::AnyValue;

/// The data of one journey: a map from string keys to values.
///
/// It starts with the instance's initial data. At the end of a journey that succeeded or failed,
/// the result carries it as the journey's output. Its keys are listed in order.
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
}

#[cfg(test)]
mod tests {
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
}
