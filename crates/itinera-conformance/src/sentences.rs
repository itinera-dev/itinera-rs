//! The sentences of the catalogue, grouped as the catalogue groups them. A sentence used only
//! by scenarios that Rust makes impossible to express is not defined, since those never run.

mod building;
mod data;
mod hooks;
mod input_adapters;
mod journey_ids;
mod policies;
mod reporters;

use std::str::FromStr;

use crate::model::ModelError;

/// One or more names, each in double quotes, separated by commas: `"a", "b"`.
#[derive(Debug, PartialEq, cucumber::Parameter)]
#[param(name = "names", regex = r#""[^"]*"(?:, "[^"]*")*"#)]
struct Names(Vec<String>);

impl FromStr for Names {
    type Err = ModelError;

    fn from_str(text: &str) -> Result<Self, ModelError> {
        text.split(", ")
            .map(unquoted)
            .collect::<Result<_, _>>()
            .map(Self)
    }
}

fn unquoted(name: &str) -> Result<String, ModelError> {
    name.strip_prefix('"')
        .and_then(|name| name.strip_suffix('"'))
        .map(str::to_owned)
        .ok_or_else(|| ModelError::Cell("name", name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_read_from_their_quotes_in_order() {
        assert_eq!(
            r#""audit", "metrics""#.parse::<Names>().unwrap(),
            Names(vec!["audit".to_owned(), "metrics".to_owned()])
        );
        assert_eq!(
            r#""audit""#.parse::<Names>().unwrap(),
            Names(vec!["audit".to_owned()])
        );
    }
}
