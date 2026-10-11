//! The scenario's services: the types of service the cases define, and the scripted providers
//! that make them, in the mode of the executor that runs them.

mod provider;
mod scope;

use std::num::NonZeroU32;

use itinera::error::Error;
use itinera::mode::Synchronous;
use itinera::policy::{Provider, Requested};

pub(crate) use provider::{Awaiting, ScriptedProvider, provided_now};
pub(crate) use scope::Scope;

use crate::declaration::ScriptedWorkflow;

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

impl Meter for Made {
    fn reading(&self) -> Result<i64, Error> {
        self.used(i64::from(self.number.get()))
    }
}

/// A type of service, as a scripted provider supplies it.
pub(crate) trait Supplied: Send + Sync + 'static {
    fn supplied(made: Made) -> Box<Self>;
}

impl Supplied for dyn Labeller {
    fn supplied(made: Made) -> Box<Self> {
        Box::new(made)
    }
}

impl Supplied for dyn Meter {
    fn supplied(made: Made) -> Box<Self> {
        Box::new(made)
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
        let labeller = <dyn Labeller>::supplied(made(failure));
        assert_eq!(labeller.label().ok().as_deref(), label);
    }

    #[test]
    fn a_meters_reading_is_its_number_among_the_instances_its_provider_made() {
        let meter = <dyn Meter>::supplied(made(None));
        assert_eq!(meter.reading().unwrap(), 2);
    }
}
