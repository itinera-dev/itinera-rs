//! What a scripted step and a scripted rescue step have in common when they are declared and
//! built: they declare inputs and services, and take their inputs, services and both handles.

use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::mode::{Asynchronous, Synchronous};
use itinera::service::Service;
use itinera::step::{
    AsyncStepReporter, InputToken, RescueNeeds, RescueResolved, Resolved, StepNeeds, StepReporter,
};

/// What declares a step's inputs and services: a step's needs or a rescue step's.
pub(super) trait Needs: Sized {
    fn input(self, input: &impl InputToken) -> Self;

    fn service<T: ?Sized + Send + Sync + 'static>(self, service: &Service<T>) -> Self;
}

/// Implements `Needs` for a kind of needs, by calling its own methods.
macro_rules! needs {
    ($needs:ident) => {
        impl Needs for $needs {
            fn input(self, input: &impl InputToken) -> Self {
                Self::input(self, input)
            }

            fn service<T: ?Sized + Send + Sync + 'static>(self, service: &Service<T>) -> Self {
                Self::service(self, service)
            }
        }
    };
}

needs!(StepNeeds);
needs!(RescueNeeds);

/// What a step is built from, for an attempt: a step's or a rescue step's resolved requests, in
/// the workflow's mode.
pub(super) trait Builds<'a> {
    /// The step reporter of the workflow's mode.
    type Reporter;

    fn input<K: InputToken>(&mut self, input: &K) -> Result<K::Given, Error>;

    fn service<T: ?Sized + 'static>(&self, service: &Service<T>) -> Result<&'a T, Error>;

    fn contributor(&mut self) -> Result<&'a mut Contributor, Error>;

    fn reporter(&mut self) -> Result<Self::Reporter, Error>;
}

/// Implements `Builds` for a kind of resolved requests in one mode, by calling its own methods.
macro_rules! builds {
    ($resolved:ident, $mode:ty, $reporter:ident) => {
        impl<'a> Builds<'a> for $resolved<'a, $mode> {
            type Reporter = $reporter<'a>;

            fn input<K: InputToken>(&mut self, input: &K) -> Result<K::Given, Error> {
                Self::input(self, input)
            }

            fn service<T: ?Sized + 'static>(&self, service: &Service<T>) -> Result<&'a T, Error> {
                Self::service(self, service)
            }

            fn contributor(&mut self) -> Result<&'a mut Contributor, Error> {
                Self::contributor(self)
            }

            fn reporter(&mut self) -> Result<$reporter<'a>, Error> {
                Self::reporter(self)
            }
        }
    };
}

builds!(Resolved, Synchronous, StepReporter);
builds!(Resolved, Asynchronous, AsyncStepReporter);
builds!(RescueResolved, Synchronous, StepReporter);
builds!(RescueResolved, Asynchronous, AsyncStepReporter);
