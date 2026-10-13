//! The services the scenario's steps request, and the service providers that make them.

use std::str::FromStr;

use serde_json::Value;

use super::{HookRequest, Level, ModelError};

/// A type of service the cases define: a `labeller`, whose one operation returns its instance's
/// label, or a `meter`, whose one operation returns an integer reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, cucumber::Parameter, derive_more::FromStr)]
#[param(name = "service", regex = "labeller|meter")]
#[from_str(rename_all = "lowercase")]
pub(crate) enum ServiceType {
    Labeller,
    Meter,
}

/// An implementation of `labeller` the cases define, which a provider makes its instances of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, cucumber::Parameter, derive_more::Display)]
#[param(name = "implementation", regex = "first labeller|second labeller")]
pub(crate) enum Implementation {
    #[default]
    #[display("first labeller")]
    FirstLabeller,
    #[display("second labeller")]
    SecondLabeller,
}

impl FromStr for Implementation {
    type Err = ModelError;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "first labeller" => Ok(Self::FirstLabeller),
            "second labeller" => Ok(Self::SecondLabeller),
            _ => Err(ModelError::UnknownImplementation(name.to_owned())),
        }
    }
}

/// How long an instance a provider makes is lent: for the journey, or for one attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scope {
    Journey,
    Attempt,
}

/// A service provider the workflow declares, and what it does on every call.
#[derive(Debug)]
pub(crate) struct Provider {
    pub(crate) name: String,
    pub(crate) key: String,
    pub(crate) service_type: ServiceType,
    pub(crate) scope: Scope,
    /// The steps it is attached to; attached to none, it serves every step that has no provider
    /// of its own for its key.
    pub(crate) steps: Vec<String>,
    /// Its requests, in the order written.
    pub(crate) requests: Vec<HookRequest>,
    /// The key of the data from the workflow whose value labels each instance it makes, instead
    /// of its name and count.
    pub(crate) labelled_by: Option<String>,
    pub(crate) failure: Option<ProviderFailure>,
    /// What it does before returning, in the order written.
    pub(crate) actions: Vec<ProviderAction>,
    /// The message the operation of every instance it makes fails with.
    pub(crate) fails_when_used: Option<String>,
    /// The implementation it makes its instances of, when it supplies a `labeller`.
    pub(crate) implementation: Implementation,
    /// The data it supplies with each instance.
    pub(crate) data: Option<Value>,
}

impl Provider {
    pub(crate) fn new(name: String, key: String, service_type: ServiceType, scope: Scope) -> Self {
        Self {
            name,
            key,
            service_type,
            scope,
            steps: Vec::new(),
            requests: Vec::new(),
            labelled_by: None,
            failure: None,
            actions: Vec::new(),
            fails_when_used: None,
            implementation: Implementation::default(),
            data: None,
        }
    }

    pub(crate) fn is_named(&self, name: &str) -> bool {
        self.name == name
    }
}

/// Something a provider does on every call, before it returns.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ProviderAction {
    Emit(Level, String),
    SetGlobalMetadata { key: String, value: Value },
}

impl ProviderAction {
    /// The value this action sets the key of global metadata to, if it sets it.
    pub(crate) fn global_metadata_for(&self, key: &str) -> Option<&Value> {
        match self {
            Self::SetGlobalMetadata { key: name, value } if name == key => Some(value),
            _ => None,
        }
    }
}

/// How a provider fails: with this message, on every call or only on a call where it received
/// a string for a request of data from the workflow.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProviderFailure {
    pub(crate) message: String,
    /// The key of the request, and the string received for it.
    pub(crate) when: Option<(String, String)>,
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::a_labeller("labeller", ServiceType::Labeller)]
    #[case::a_meter("meter", ServiceType::Meter)]
    fn a_service_type_is_read_by_its_name(#[case] name: &str, #[case] service_type: ServiceType) {
        assert_eq!(name.parse::<ServiceType>().unwrap(), service_type);
    }

    #[rstest]
    #[case::the_first_labeller("first labeller", Implementation::FirstLabeller)]
    #[case::the_second_labeller("second labeller", Implementation::SecondLabeller)]
    fn an_implementation_is_read_and_written_by_its_name(
        #[case] name: &str,
        #[case] implementation: Implementation,
    ) {
        assert_eq!(name.parse::<Implementation>().unwrap(), implementation);
        assert_eq!(implementation.to_string(), name);
    }
}
