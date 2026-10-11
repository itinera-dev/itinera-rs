//! One input a step declares, and whether the step needs it.

use crate::value::{AnyValue, Value};

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
    /// A value of type `T` under the key, without which nothing can be built.
    pub(crate) fn required<T: Value>(key: &'static str) -> Self {
        Self {
            key,
            requirement: Requirement::Required,
            accepts: AnyValue::is::<T>,
        }
    }

    /// A value of type `T` under the key, which may be absent.
    pub(crate) fn optional<T: Value>(key: &'static str) -> Self {
        Self {
            key,
            requirement: Requirement::Optional,
            accepts: AnyValue::is::<T>,
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
