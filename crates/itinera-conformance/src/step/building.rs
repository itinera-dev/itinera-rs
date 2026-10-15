//! What a scripted step and a scripted rescue step have in common when they are declared and
//! built: they declare inputs and services, and take their inputs, services and both handles.

use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::mode::{Asynchronous, Synchronous};
use itinera::service::Service;
use itinera::step::{
    AsyncStepReporter, Input, OptionalInput, RescueNeeds, RescueResolved, Resolved, StepNeeds,
    StepReporter,
};
use itinera::value::Value as Storable;

/// What declares a step's inputs and services: a step's needs or a rescue step's.
pub(super) trait Needs: Sized {
    fn input<T: Storable>(self, input: &Input<T>) -> Self;

    fn optional_input<T: Storable>(self, input: &OptionalInput<T>) -> Self;

    fn service<T: ?Sized + Send + Sync + 'static>(self, service: &Service<T>) -> Self;
}

/// Implements `Needs` for a kind of needs, by calling its own methods.
macro_rules! needs {
    ($needs:ident) => {
        impl Needs for $needs {
            fn input<T: Storable>(self, input: &Input<T>) -> Self {
                Self::input(self, input)
            }

            fn optional_input<T: Storable>(self, input: &OptionalInput<T>) -> Self {
                Self::optional_input(self, input)
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

    fn input<T: Storable>(&mut self, input: &Input<T>) -> Result<T, Error>;

    fn optional_input<T: Storable>(&mut self, input: &OptionalInput<T>)
    -> Result<Option<T>, Error>;

    fn service<T: ?Sized + 'static>(&self, service: &Service<T>) -> Result<&'a T, Error>;

    fn contributor(&mut self) -> Result<&'a mut Contributor, Error>;

    fn reporter(&mut self) -> Result<Self::Reporter, Error>;
}

/// Implements `Builds` for a kind of resolved requests in one mode, by calling its own methods.
macro_rules! builds {
    ($resolved:ident, $mode:ty, $reporter:ident) => {
        impl<'a> Builds<'a> for $resolved<'a, $mode> {
            type Reporter = $reporter<'a>;

            fn input<T: Storable>(&mut self, input: &Input<T>) -> Result<T, Error> {
                Self::input(self, input)
            }

            fn optional_input<T: Storable>(
                &mut self,
                input: &OptionalInput<T>,
            ) -> Result<Option<T>, Error> {
                Self::optional_input(self, input)
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
