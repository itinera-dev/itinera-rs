//! The scenario's services: the types of service the cases define, and the scripted providers
//! that make them, in the mode of the executor that runs them.

mod provider;
mod scope;

use std::any;
use std::num::NonZeroU32;

use itinera::error::Error;
use itinera::mode::Synchronous;
use itinera::policy::{Provider, Requested};
use itinera::service::{Provided, provide};
use serde_json::Value;

pub(crate) use provider::{Awaiting, ScriptedProvider, provided_now};
pub(crate) use scope::Scope;

use crate::declaration::ScriptedWorkflow;
use crate::model::Implementation;

/// What a scripted provider of scope `S` requested, in the mode `M`.
pub(crate) type Got<'a, S, M = Synchronous> = Requested<'a, ScriptedWorkflow, Provider<S>, M>;

/// A service of type `labeller`.
pub(crate) trait Labeller: Send + Sync {
    /// The instance's label.
    fn label(&self) -> Result<String, Error>;
}

/// A service of type `meter`, which a step requesting a `labeller` never accepts.
pub(crate) trait Meter: Send + Sync {
    /// The instance's reading.
    fn reading(&self) -> Result<i64, Error>;
}

/// An instance a scripted provider made, of either type: its label, its number among the
/// instances its provider made in the journey, which is a meter's reading, and the message its
/// operation fails with, if it fails.
#[derive(Debug)]
pub(crate) struct Made {
    label: String,
    number: NonZeroU32,
    failure: Option<String>,
}

impl Made {
    /// What the operation returns, unless it fails.
    fn used<T>(&self, returned: T) -> Result<T, Error> {
        match &self.failure {
            Some(message) => Err(Error::msg(message.clone())),
            None => Ok(returned),
        }
    }
}

impl Labeller for Made {
    fn label(&self) -> Result<String, Error> {
        self.used(self.label.clone())
    }
}

/// The implementation `first labeller`, which providers make unless the scenario says otherwise.
#[derive(Debug)]
pub(crate) struct FirstLabeller {
    made: Made,
}

impl Labeller for FirstLabeller {
    fn label(&self) -> Result<String, Error> {
        self.made.label()
    }
}

/// The implementation `second labeller`, another type than `first labeller` labelling alike.
#[derive(Debug)]
pub(crate) struct SecondLabeller {
    made: Made,
}

impl Labeller for SecondLabeller {
    fn label(&self) -> Result<String, Error> {
        self.made.label()
    }
}

/// The implementation an event names, as the cases write it: the name of one of the cases'
/// implementations when it is the name Rust gives that implementation's type, and otherwise
/// that name itself.
pub(crate) fn implementation_named(implementation: itinera::service::Implementation) -> String {
    let name: &str = implementation.as_ref();
    if name == any::type_name::<FirstLabeller>() {
        Implementation::FirstLabeller.to_string()
    } else if name == any::type_name::<SecondLabeller>() {
        Implementation::SecondLabeller.to_string()
    } else {
        name.to_owned()
    }
}

impl Meter for Made {
    fn reading(&self) -> Result<i64, Error> {
        self.used(i64::from(self.number.get()))
    }
}

/// A type of service, as a scripted provider supplies it: of the implementation the scenario
/// chose, with the data the provider supplies, if any.
pub(crate) trait Supplied: Send + Sync + 'static {
    fn supplied(made: Made, implementation: Implementation, data: Option<Value>) -> Provided<Self>;
}

impl Supplied for dyn Labeller {
    fn supplied(made: Made, implementation: Implementation, data: Option<Value>) -> Provided<Self> {
        match (implementation, data) {
            (Implementation::FirstLabeller, None) => provide!(FirstLabeller { made }),
            (Implementation::FirstLabeller, Some(data)) => provide!(FirstLabeller { made }, data),
            (Implementation::SecondLabeller, None) => provide!(SecondLabeller { made }),
            (Implementation::SecondLabeller, Some(data)) => provide!(SecondLabeller { made }, data),
        }
    }
}

/// A `meter` has no implementations of its own in the cases: every one is the runner's `Made`.
impl Supplied for dyn Meter {
    fn supplied(made: Made, _: Implementation, data: Option<Value>) -> Provided<Self> {
        match data {
            None => provide!(made),
            Some(data) => provide!(made, data),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn made(failure: Option<&str>) -> Made {
        Made {
            label: "stock 2".to_owned(),
            number: NonZeroU32::new(2).unwrap(),
            failure: failure.map(str::to_owned),
        }
    }

    #[rstest]
    #[case::that_works(None, Some("stock 2"))]
    #[case::whose_services_fail(Some("card network down"), None)]
    fn a_labeller_tells_its_label_unless_its_provider_makes_failing_services(
        #[case] failure: Option<&str>,
        #[case] label: Option<&str>,
    ) {
        let labeller = <dyn Labeller>::supplied(made(failure), Implementation::default(), None);
        assert_eq!(labeller.service().label().ok().as_deref(), label);
    }

    #[test]
    fn a_meters_reading_is_its_number_among_the_instances_its_provider_made() {
        let meter = <dyn Meter>::supplied(made(None), Implementation::default(), None);
        assert_eq!(meter.service().reading().unwrap(), 2);
    }

    #[rstest]
    #[case::the_first_labeller(Implementation::FirstLabeller)]
    #[case::the_second_labeller(Implementation::SecondLabeller)]
    fn an_instance_is_named_by_the_implementation_the_scenario_chose(
        #[case] implementation: Implementation,
    ) {
        let labeller = <dyn Labeller>::supplied(made(None), implementation, None);
        assert_eq!(
            implementation_named(labeller.implementation()),
            implementation.to_string()
        );
    }
}
