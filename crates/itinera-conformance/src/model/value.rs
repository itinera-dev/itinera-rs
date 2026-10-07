//! Values and types as the cases write them.

use std::str::FromStr;

use serde_json::Value;

use super::ModelError;

/// A type of the neutral vocabulary the cases use, which the runner maps to a Rust type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, cucumber::Parameter)]
#[param(name = "type", regex = "string|integer|number|boolean|list|object")]
pub(crate) enum ValueType {
    String,
    Integer,
    Number,
    Boolean,
    List,
    Object,
}

impl FromStr for ValueType {
    type Err = ModelError;

    fn from_str(name: &str) -> Result<Self, ModelError> {
        Ok(match name {
            "string" => Self::String,
            "integer" => Self::Integer,
            "number" => Self::Number,
            "boolean" => Self::Boolean,
            "list" => Self::List,
            "object" => Self::Object,
            _ => return Err(ModelError::UnknownType(name.to_owned())),
        })
    }
}

/// Reads a value written as JSON.
pub(crate) fn json(text: &str) -> Result<Value, ModelError> {
    serde_json::from_str(text).map_err(|_| ModelError::NotJson(text.to_owned()))
}

/// Reads a JSON object as keys and values, in the order written.
pub(crate) fn entries(text: &str) -> Result<Vec<(String, Value)>, ModelError> {
    match json(text)? {
        Value::Object(object) => Ok(object.into_iter().collect()),
        _ => Err(ModelError::NotAnObject(text.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn every_type_of_the_neutral_vocabulary_is_read() {
        let names = ["string", "integer", "number", "boolean", "list", "object"];
        let types = names.map(|name| name.parse::<ValueType>().unwrap());
        assert_eq!(
            types,
            [
                ValueType::String,
                ValueType::Integer,
                ValueType::Number,
                ValueType::Boolean,
                ValueType::List,
                ValueType::Object,
            ]
        );
        assert!("float".parse::<ValueType>().is_err());
    }

    #[test]
    fn a_number_without_a_fraction_is_an_integer_and_any_other_is_not() {
        assert!(json("42").unwrap().is_i64());
        assert!(json("4.0").unwrap().is_f64());
        assert!(json("4.5").unwrap().is_f64());
    }

    #[test]
    fn an_objects_entries_keep_the_order_written() {
        assert_eq!(
            entries(r#"{"z": 1, "a": "x"}"#).unwrap(),
            [("z".to_owned(), json!(1)), ("a".to_owned(), json!("x"))]
        );
        assert!(entries("[1]").is_err());
    }
}
