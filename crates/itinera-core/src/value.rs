//! Values: data that can be serialized, the only data a data bag, an event or a reason holds.

use std::any::{self, Any};
use std::fmt;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Data that could be serialized: the only kind of data a data bag, the data of an event or the
/// details of a reason can hold.
///
/// Every type that is `Serialize + DeserializeOwned + Clone + Send + Sync + 'static` is a value,
/// so closures, function pointers and handles never are. Nothing is serialized by itinera: the
/// bounds only make sure that something could serialize it.
///
/// Whoever receives a value gets its own clone. A type whose `Clone` shares mutable state, such as
/// `Arc<Mutex<_>>`, lets one receiver change what another sees, and should not be used as a value.
///
/// # Examples
///
/// ```
/// use itinera::value::Value;
///
/// fn accepts<T: Value>(_: T) {}
///
/// accepts(42_i64);
/// accepts("R-1".to_string());
/// accepts(vec![1.5_f64, 2.5]);
/// ```
pub trait Value: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {}

impl<T> Value for T where T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {}

/// A [`Value`] whose type has been erased, which can still be cloned, serialized, and read back
/// as its exact type.
///
/// Reading names the exact type: any other type, even `i32` for an `i64`, reads nothing.
///
/// # Examples
///
/// ```
/// use itinera::value::AnyValue;
///
/// let value = AnyValue::new(42_i64);
/// assert_eq!(value.downcast_ref::<i64>(), Some(&42));
/// assert_eq!(value.downcast_ref::<i32>(), None);
/// ```
pub struct AnyValue {
    value: Box<dyn Erased>,
}

impl AnyValue {
    /// Erases the type of a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::value::AnyValue;
    ///
    /// let value = AnyValue::new("R-1".to_string());
    /// assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("R-1"));
    /// ```
    pub fn new<T: Value>(value: T) -> Self {
        Self {
            value: Box::new(value),
        }
    }

    /// The value, if it is of type `T` exactly.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::value::AnyValue;
    ///
    /// let value = AnyValue::new(true);
    /// assert_eq!(value.downcast_ref::<bool>(), Some(&true));
    /// assert_eq!(value.downcast_ref::<String>(), None);
    /// ```
    pub fn downcast_ref<T: Value>(&self) -> Option<&T> {
        self.value.as_any().downcast_ref()
    }

    /// The name of the value's type, for diagnostics only: it is not guaranteed to be stable.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::value::AnyValue;
    ///
    /// assert!(AnyValue::new(42_i64).type_name().contains("i64"));
    /// ```
    pub fn type_name(&self) -> &'static str {
        self.value.type_name()
    }
}

impl Clone for AnyValue {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone_box(),
        }
    }
}

impl fmt::Debug for AnyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AnyValue").field(&self.type_name()).finish()
    }
}

impl Serialize for AnyValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        erased_serde::serialize(self.value.as_ref(), serializer)
    }
}

trait Erased: erased_serde::Serialize + Send + Sync {
    fn clone_box(&self) -> Box<dyn Erased>;
    fn as_any(&self) -> &dyn Any;
    fn type_name(&self) -> &'static str;
}

impl<T: Value> Erased for T {
    fn clone_box(&self) -> Box<dyn Erased> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        any::type_name::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clone_of_an_erased_value_is_independent_of_the_original() {
        let original = AnyValue::new(vec![1_i64, 2]);
        let clone = original.clone();
        drop(original);
        assert_eq!(clone.downcast_ref::<Vec<i64>>(), Some(&vec![1, 2]));
    }

    #[test]
    fn an_erased_value_serializes_as_the_value_itself() {
        let value = AnyValue::new(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(serde_json::to_string(&value).unwrap(), r#"["a","b"]"#);
    }

    #[test]
    fn an_erased_value_is_read_only_as_its_exact_type() {
        let value = AnyValue::new(7_i64);
        assert!(value.downcast_ref::<i32>().is_none());
        assert!(value.downcast_ref::<u64>().is_none());
        assert_eq!(value.downcast_ref::<i64>(), Some(&7));
    }
}
