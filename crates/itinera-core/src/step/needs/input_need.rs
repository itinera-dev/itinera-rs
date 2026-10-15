//! One input a step declares, and whether the step needs it.

use crate::step::InputToken;
use crate::value::AnyValue;

/// Whether a step needs an input to be built, or may be built without it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Requirement {
    Required,
    Optional,
}

/// One declared input, as the executor resolves it.
#[derive(Clone, derive_more::Debug)]
pub(crate) struct InputNeed {
    key: &'static str,
    requirement: Requirement,
    #[debug(skip)]
    accepts: fn(&AnyValue) -> bool,
}

impl InputNeed {
    /// The input a token names: required for an `Input`, optional for an `OptionalInput`.
    pub(crate) fn of<K: InputToken>(token: &K) -> Self {
        Self {
            key: token.key(),
            requirement: if K::OPTIONAL {
                Requirement::Optional
            } else {
                Requirement::Required
            },
            accepts: AnyValue::is::<K::Value>,
        }
    }

    pub(crate) fn key(&self) -> &'static str {
        self.key
    }

    pub(crate) fn requirement(&self) -> Requirement {
        self.requirement
    }

    /// Whether a value has the type the input declares.
    pub(crate) fn accepts(&self, value: &AnyValue) -> bool {
        (self.accepts)(value)
    }
}
