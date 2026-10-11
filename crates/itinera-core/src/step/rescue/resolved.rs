//! What the executor resolved for a rescue step, from which its factory builds it.

use std::any;

use crate::error::Error;
use crate::journey::Contributor;
use crate::mode::Synchronous;
#[cfg(feature = "unstable")]
use crate::service::Service;
use crate::step::{Got, Input, OptionalInput, Reason, Resolved, StepReporter};
use crate::value::{AnyValue, Value};

/// What the executor resolved for a rescue step, from which its factory builds it: its inputs,
/// what the failed attempt of the step it rescues left, and its handles.
///
/// Each value is the rescue step's own: nothing it does to one reaches the data bag or anyone
/// else. `M` is the workflow's mode.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{Input, RescueResolved};
///
/// const ACCOUNT: Input<String> = Input::new("account");
///
/// fn account(got: &mut RescueResolved<'_>) -> Result<String, Error> {
///     got.from_rescued_step(&ACCOUNT)
/// }
/// ```
#[derive(derive_more::Debug)]
pub struct RescueResolved<'a, M = Synchronous> {
    resolved: Resolved<'a, M>,
}

impl<'a, M> RescueResolved<'a, M> {
    pub(crate) fn new(got: Got<'a>) -> Self {
        Self {
            resolved: Resolved::new(got),
        }
    }

    /// Takes the value of a required input from the data bag.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare this input with this type, or its value was
    /// already taken. Building fails with that error, and the journey is aborted with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{Input, RescueResolved};
    ///
    /// fn client(got: &mut RescueResolved<'_>) -> Result<String, Error> {
    ///     got.input(&Input::new("client"))
    /// }
    /// ```
    pub fn input<T: Value>(&mut self, input: &Input<T>) -> Result<T, Error> {
        self.resolved.input(input)
    }

    /// Takes the value of an optional input from the data bag: `None` when there was none.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare this input with this type, or it was already
    /// taken. Building fails with that error, and the journey is aborted with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{OptionalInput, RescueResolved};
    ///
    /// fn region(got: &mut RescueResolved<'_>) -> Result<Option<String>, Error> {
    ///     got.optional_input(&OptionalInput::new("region"))
    /// }
    /// ```
    pub fn optional_input<T: Value>(
        &mut self,
        input: &OptionalInput<T>,
    ) -> Result<Option<T>, Error> {
        self.resolved.optional_input(input)
    }

    /// Takes the data the failed attempt contributed under the input's key, as the rescue step
    /// declared it, required.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare it as required with this type, or it was
    /// already taken. Building fails with that error, and the journey is aborted with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{Input, RescueResolved};
    ///
    /// fn account(got: &mut RescueResolved<'_>) -> Result<String, Error> {
    ///     got.from_rescued_step(&Input::new("account"))
    /// }
    /// ```
    pub fn from_rescued_step<T: Value>(&mut self, input: &Input<T>) -> Result<T, Error> {
        self.resolved
            .rescued()
            .contributed
            .take(input.key)
            .and_then(AnyValue::downcast)
            .ok_or_else(|| undeclared::<T>(input.key))
    }

    /// Takes the data the failed attempt contributed under the input's key, as the rescue step
    /// declared it, optional: `None` when the attempt contributed nothing there.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare it as optional with this type, or it was
    /// already taken. Building fails with that error, and the journey is aborted with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{OptionalInput, RescueResolved};
    ///
    /// fn account(got: &mut RescueResolved<'_>) -> Result<Option<String>, Error> {
    ///     got.optional_from_rescued_step(&OptionalInput::new("account"))
    /// }
    /// ```
    pub fn optional_from_rescued_step<T: Value>(
        &mut self,
        input: &OptionalInput<T>,
    ) -> Result<Option<T>, Error> {
        match self.resolved.rescued().contributed.take_optional(input.key) {
            Some(None) => Ok(None),
            Some(Some(value)) => value
                .downcast()
                .map(Some)
                .ok_or_else(|| undeclared::<T>(input.key)),
            None => Err(undeclared::<T>(input.key)),
        }
    }

    /// Takes the reason of the failure it rescues, as the rescue step declared it, required.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare the reason, it is absent, or it was already
    /// taken. Building fails with that error, and the journey is aborted with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{Reason, RescueResolved};
    ///
    /// fn reason(got: &mut RescueResolved<'_>) -> Result<Reason, Error> {
    ///     got.reason()
    /// }
    /// ```
    pub fn reason(&mut self) -> Result<Reason, Error> {
        self.resolved
            .rescued()
            .reason
            .take()
            .flatten()
            .ok_or_else(|| undeclared_failure("the failure reason as required"))
    }

    /// Takes the reason of the failure it rescues, as the rescue step declared it, optional:
    /// `None` when the attempt reported no failure.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare the reason, or it was already taken. Building
    /// fails with that error, and the journey is aborted with `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{Reason, RescueResolved};
    ///
    /// fn reason(got: &mut RescueResolved<'_>) -> Result<Option<Reason>, Error> {
    ///     got.optional_reason()
    /// }
    /// ```
    pub fn optional_reason(&mut self) -> Result<Option<Reason>, Error> {
        self.resolved
            .rescued()
            .reason
            .take()
            .ok_or_else(|| undeclared_failure("the failure reason"))
    }

    /// Takes the error that ended the attempt it rescues, as the rescue step declared it,
    /// required.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare the error, it is absent, or it was already
    /// taken. Building fails with that error, and the journey is aborted with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::RescueResolved;
    ///
    /// fn error(got: &mut RescueResolved<'_>) -> Result<Error, Error> {
    ///     got.error()
    /// }
    /// ```
    pub fn error(&mut self) -> Result<Error, Error> {
        self.resolved
            .rescued()
            .error
            .take()
            .flatten()
            .ok_or_else(|| undeclared_failure("the error as required"))
    }

    /// Takes the error that ended the attempt it rescues, as the rescue step declared it,
    /// optional: `None` when the attempt did not end in an abnormal termination.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare the error, or it was already taken. Building
    /// fails with that error, and the journey is aborted with `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::RescueResolved;
    ///
    /// fn error(got: &mut RescueResolved<'_>) -> Result<Option<Error>, Error> {
    ///     got.optional_error()
    /// }
    /// ```
    pub fn optional_error(&mut self) -> Result<Option<Error>, Error> {
        self.resolved
            .rescued()
            .error
            .take()
            .ok_or_else(|| undeclared_failure("the error"))
    }

    /// Takes the rescue step's contributor.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare a contributor, or it was already taken.
    /// Building fails with that error, and the journey is aborted with `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::journey::Contributor;
    /// use itinera::step::RescueResolved;
    ///
    /// fn contributor<'a>(got: &mut RescueResolved<'a>) -> Result<Contributor<'a>, Error> {
    ///     got.contributor()
    /// }
    /// ```
    pub fn contributor(&mut self) -> Result<Contributor<'a>, Error> {
        self.resolved.contributor()
    }

    /// The service the rescue step declared under the token's key, lent for its attempt.
    ///
    /// The rescue step never knows which provider made it, whether another step shares it, or
    /// how long it lives. It may take a service more than once.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare this service with this type. Building fails
    /// with that error, and the journey is aborted with `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::service::Service;
    /// use itinera::step::RescueResolved;
    ///
    /// trait PaymentGateway: Send + Sync {}
    ///
    /// const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");
    ///
    /// fn payments<'a>(got: &RescueResolved<'a>) -> Result<&'a dyn PaymentGateway, Error> {
    ///     got.service(&PAYMENTS)
    /// }
    /// ```
    #[cfg(feature = "unstable")]
    pub fn service<T: ?Sized + 'static>(&self, service: &Service<T>) -> Result<&'a T, Error> {
        self.resolved.service(service)
    }
}

impl<'a> RescueResolved<'a, Synchronous> {
    /// Takes the rescue step's reporter.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare a reporter, or it was already taken. Building
    /// fails with that error, and the journey is aborted with `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{RescueResolved, StepReporter};
    ///
    /// fn reporter<'a>(got: &mut RescueResolved<'a>) -> Result<StepReporter<'a>, Error> {
    ///     got.reporter()
    /// }
    /// ```
    pub fn reporter(&mut self) -> Result<StepReporter<'a>, Error> {
        self.resolved.reporter()
    }
}

#[cfg(feature = "async")]
impl<'a> RescueResolved<'a, crate::mode::Asynchronous> {
    /// Takes the rescue step's reporter.
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare a reporter, or it was already taken. Building
    /// fails with that error, and the journey is aborted with `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::step::{AsyncStepReporter, RescueResolved};
    ///
    /// fn reporter<'a>(
    ///     got: &mut RescueResolved<'a, Asynchronous>,
    /// ) -> Result<AsyncStepReporter<'a>, Error> {
    ///     got.reporter()
    /// }
    /// ```
    pub fn reporter(&mut self) -> Result<crate::step::AsyncStepReporter<'a>, Error> {
        self.resolved.reporter()
    }
}

fn undeclared<T>(key: &str) -> Error {
    Error::msg(format!(
        "the rescue step does not declare the data \"{key}\" of type {} from the step it \
         rescues, or took it already",
        any::type_name::<T>()
    ))
}

fn undeclared_failure(what: &str) -> Error {
    Error::msg(format!(
        "the rescue step does not declare {what}, or took it already"
    ))
}
