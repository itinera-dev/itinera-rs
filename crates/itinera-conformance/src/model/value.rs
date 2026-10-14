//! Values and types as the cases write them.

use serde_json::Value;

use super::ModelError;

/// A type of the neutral vocabulary the cases use, which the runner maps to a Rust type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, cucumber::Parameter, derive_more::FromStr)]
#[param(name = "type", regex = "string|integer|number|boolean|list|object")]
#[from_str(rename_all = "lowercase")]
pub(crate) enum ValueType {
    String,
    Integer,
    Number,
    Boolean,
    List,
    Object,
}

impl ValueType {
    pub(crate) const ALL: [Self; 6] = [
        Self::String,
        Self::Integer,
        Self::Number,
        Self::Boolean,
        Self::List,
        Self::Object,
    ];
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
    use rstest::rstest;
    use serde_json::json;

    use super::*;

    fn parsed(name: &str) -> ValueType {
        name.parse().unwrap()
    }

    #[rstest]
    #[case::string("string", ValueType::String)]
    #[case::integer("integer", ValueType::Integer)]
    #[case::number("number", ValueType::Number)]
    #[case::boolean("boolean", ValueType::Boolean)]
    #[case::list("list", ValueType::List)]
    #[case::object("object", ValueType::Object)]
    fn every_type_of_the_neutral_vocabulary_is_read(#[case] name: &str, #[case] read: ValueType) {
        assert_eq!(parsed(name), read);
    }

    #[test]
    fn a_type_outside_the_neutral_vocabulary_is_refused() {
        assert!("float".parse::<ValueType>().is_err());
    }

    #[rstest]
    #[case::a_whole_number("42", true)]
    #[case::a_zero_fraction("4.0", false)]
    #[case::a_fraction("4.5", false)]
    fn a_number_without_a_fraction_is_an_integer_and_any_other_is_not(
        #[case] text: &str,
        #[case] integer: bool,
    ) {
        assert_eq!(json(text).unwrap().is_i64(), integer);
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
