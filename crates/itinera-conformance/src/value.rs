//! Values the cases write as JSON, given to itinera as the Rust types the neutral vocabulary maps
//! them to.

use itinera::error::Error;
use itinera::value::{AnyValue, Value as Storable};
use serde_json::{Map, Value};

use crate::declaration::leaked;
use crate::model::{ModelError, ValueType};

/// A value of one of the vocabulary's types, held as the Rust type that type maps to: `string`
/// as `String`, `integer` as `i64`, `number` as `f64`, `boolean` as `bool`, `list` as a vector
/// of JSON values and `object` as a JSON map.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Typed {
    String(String),
    Integer(i64),
    Number(f64),
    Boolean(bool),
    List(Vec<Value>),
    Object(Map<String, Value>),
}

/// Data a step or a hook requests under a key, of a type of the vocabulary, required or optional.
#[derive(Debug)]
pub(crate) struct Data {
    pub(crate) key: &'static str,
    pub(crate) value_type: ValueType,
    pub(crate) optional: bool,
}

impl Data {
    pub(crate) fn of(key: &str, value_type: ValueType, optional: bool) -> Self {
        Self {
            key: leaked(key),
            value_type,
            optional,
        }
    }
}

/// A value, as the cases write it.
pub(crate) fn json<T: Storable>(value: T) -> Result<Value, Error> {
    Ok(serde_json::to_value(value)?)
}

/// What takes a value of whichever type it is, such as a contributor, and what that gives.
pub(crate) trait Takes {
    type Output;

    fn take<T: Storable>(self, value: T) -> Self::Output;
}

/// What depends on a type of the vocabulary, such as declaring an input of that type, and what
/// that gives.
pub(crate) trait ForType {
    type Output;

    fn of<T: Storable>(self) -> Self::Output;
}

/// Gives what depends on a type of the vocabulary the Rust type that type maps to.
pub(crate) fn for_type<K: ForType>(value_type: ValueType, dependent: K) -> K::Output {
    match value_type {
        ValueType::String => dependent.of::<String>(),
        ValueType::Integer => dependent.of::<i64>(),
        ValueType::Number => dependent.of::<f64>(),
        ValueType::Boolean => dependent.of::<bool>(),
        ValueType::List => dependent.of::<Vec<Value>>(),
        ValueType::Object => dependent.of::<Map<String, Value>>(),
    }
}

impl Typed {
    /// The value written as JSON, with the type of the vocabulary its JSON form has: a number
    /// written without a fraction or an exponent is an `integer`.
    pub(crate) fn of(value: &Value) -> Result<Self, ModelError> {
        Ok(match value {
            Value::String(text) => Self::String(text.clone()),
            Value::Number(number) => match number.as_i64() {
                Some(integer) => Self::Integer(integer),
                None => Self::Number(
                    number
                        .as_f64()
                        .ok_or_else(|| ModelError::Untyped(value.to_string()))?,
                ),
            },
            Value::Bool(boolean) => Self::Boolean(*boolean),
            Value::Array(list) => Self::List(list.clone()),
            Value::Object(object) => Self::Object(object.clone()),
            Value::Null => return Err(ModelError::Untyped(value.to_string())),
        })
    }

    /// Gives the value, as its own Rust type, to what takes it.
    pub(crate) fn given_to<K: Takes>(self, taker: K) -> K::Output {
        match self {
            Self::String(value) => taker.take(value),
            Self::Integer(value) => taker.take(value),
            Self::Number(value) => taker.take(value),
            Self::Boolean(value) => taker.take(value),
            Self::List(value) => taker.take(value),
            Self::Object(value) => taker.take(value),
        }
    }

    /// The value, with its type erased as itinera holds it.
    pub(crate) fn erased(self) -> AnyValue {
        self.given_to(Erasing)
    }
}

/// Erases the type of a value.
struct Erasing;

impl Takes for Erasing {
    type Output = AnyValue;

    fn take<T: Storable>(self, value: T) -> AnyValue {
        AnyValue::new(value)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    fn is<T: Storable>(value: &AnyValue) -> bool {
        value.downcast_ref::<T>().is_some()
    }

    #[rstest]
    #[case::text_as_a_string(json!("R-1"), is::<String>)]
    #[case::a_whole_number_as_an_integer(json!(42), is::<i64>)]
    #[case::a_fraction_as_a_number(json!(4.5), is::<f64>)]
    #[case::a_boolean(json!(true), is::<bool>)]
    #[case::a_list(json!([1, "a"]), is::<Vec<Value>>)]
    #[case::an_object(json!({"ms": 1200}), is::<Map<String, Value>>)]
    fn a_value_is_given_as_the_rust_type_of_the_type_its_json_form_has(
        #[case] value: Value,
        #[case] typed: fn(&AnyValue) -> bool,
    ) {
        assert!(typed(&Typed::of(&value).unwrap().erased()));
    }

    /// Whether the value has the Rust type the dependent is given.
    struct IsOf<'v> {
        value: &'v AnyValue,
    }

    impl ForType for IsOf<'_> {
        type Output = bool;

        fn of<T: Storable>(self) -> bool {
            is::<T>(self.value)
        }
    }

    #[rstest]
    #[case::string(ValueType::String, json!("R-1"))]
    #[case::integer(ValueType::Integer, json!(42))]
    #[case::number(ValueType::Number, json!(4.5))]
    #[case::boolean(ValueType::Boolean, json!(true))]
    #[case::list(ValueType::List, json!([1, "a"]))]
    #[case::object(ValueType::Object, json!({"ms": 1200}))]
    fn a_type_maps_to_the_rust_type_its_values_are_given_as(
        #[case] value_type: ValueType,
        #[case] value: Value,
    ) {
        let erased = Typed::of(&value).unwrap().erased();
        assert!(for_type(value_type, IsOf { value: &erased }));
    }

    #[test]
    fn null_has_no_type_of_the_vocabulary() {
        assert_eq!(
            Typed::of(&Value::Null),
            Err(ModelError::Untyped("null".to_owned()))
        );
    }
}
