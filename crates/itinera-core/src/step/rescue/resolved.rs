//! What the executor resolved for a rescue step, from which its factory builds it.

use std::any;

use crate::error::Error;
use crate::journey::Contributor;
use crate::mode::Synchronous;
use crate::service::Service;
use crate::step::{Got, InputToken, Reason, Resolved, StepReporter};

/// What the executor resolved for a rescue step, from which its factory builds it: its inputs,
/// what the failed attempt of the step it rescues left, and its handles.
///
/// Each value is the rescue step's own: nothing it does to one reaches the data bag or anyone
/// else. `M` is the workflow's mode.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::step::{Input, RescueResolved};
///
/// const ACCOUNT: Input<String> = Input::new("account");
///
/// fn account(got: &mut RescueResolved<'_>) -> Result<String> {
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

    /// Takes the value of an input: the value for an [`Input`], and for an [`OptionalInput`] an
    /// `Option` of it, `None` when there was none.
    ///
    /// [`Input`]: crate::step::Input
    /// [`OptionalInput`]: crate::step::OptionalInput
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare this input's key with its type, or its value
    /// was already taken. Building fails with that error, and the journey is aborted with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::step::{Input, OptionalInput, RescueResolved};
    ///
    /// fn client(got: &mut RescueResolved<'_>) -> Result<String> {
    ///     got.input(&Input::new("client"))
    /// }
    ///
    /// fn region(got: &mut RescueResolved<'_>) -> Result<Option<String>> {
    ///     got.input(&OptionalInput::new("region"))
    /// }
    /// ```
    pub fn input<K: InputToken>(&mut self, input: &K) -> Result<K::Given, Error> {
        self.resolved.input(input)
    }

    /// Takes the data the failed attempt contributed under the input's key, as the rescue step
    /// declared it: the value for an [`Input`], and for an [`OptionalInput`] an `Option` of it,
    /// `None` when the attempt contributed nothing there.
    ///
    /// [`Input`]: crate::step::Input
    /// [`OptionalInput`]: crate::step::OptionalInput
    ///
    /// # Errors
    ///
    /// When the rescue step's needs do not declare its key with its type, or it was already taken.
    /// Building fails with that error, and the journey is aborted with
    /// `step could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Result;
    /// use itinera::step::{Input, OptionalInput, RescueResolved};
    ///
    /// fn account(got: &mut RescueResolved<'_>) -> Result<String> {
    ///     got.from_rescued_step(&Input::new("account"))
    /// }
    ///
    /// fn note(got: &mut RescueResolved<'_>) -> Result<Option<String>> {
    ///     got.from_rescued_step(&OptionalInput::new("note"))
    /// }
    /// ```
    pub fn from_rescued_step<K: InputToken>(&mut self, input: &K) -> Result<K::Given, Error> {
        let slot = self.resolved.rescued().contributed.take(input.key());
        K::given(slot).ok_or_else(|| undeclared::<K::Value>(input.key()))
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
    /// use itinera::error::Result;
    /// use itinera::step::{Reason, RescueResolved};
    ///
    /// fn reason(got: &mut RescueResolved<'_>) -> Result<Reason> {
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
    /// use itinera::error::Result;
    /// use itinera::step::{Reason, RescueResolved};
    ///
    /// fn reason(got: &mut RescueResolved<'_>) -> Result<Option<Reason>> {
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
    /// use itinera::error::{Error, Result};
    /// use itinera::step::RescueResolved;
    ///
    /// fn error(got: &mut RescueResolved<'_>) -> Result<Error> {
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
    /// use itinera::error::{Error, Result};
    /// use itinera::step::RescueResolved;
    ///
    /// fn error(got: &mut RescueResolved<'_>) -> Result<Option<Error>> {
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
    /// use itinera::error::Result;
    /// use itinera::journey::Contributor;
    /// use itinera::step::RescueResolved;
    ///
    /// fn contributor<'a>(got: &mut RescueResolved<'a>) -> Result<&'a mut Contributor> {
    ///     got.contributor()
    /// }
    /// ```
    pub fn contributor(&mut self) -> Result<&'a mut Contributor, Error> {
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
    /// use itinera::error::Result;
    /// use itinera::service::Service;
    /// use itinera::step::RescueResolved;
    ///
    /// trait PaymentGateway: Send + Sync {}
    ///
    /// const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");
    ///
    /// fn payments<'a>(got: &RescueResolved<'a>) -> Result<&'a dyn PaymentGateway> {
    ///     got.service(&PAYMENTS)
    /// }
    /// ```
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
    /// use itinera::error::Result;
    /// use itinera::step::{RescueResolved, StepReporter};
    ///
    /// fn reporter<'a>(got: &mut RescueResolved<'a>) -> Result<StepReporter<'a>> {
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
    /// use itinera::error::Result;
    /// use itinera::mode::Asynchronous;
    /// use itinera::step::{AsyncStepReporter, RescueResolved};
    ///
    /// fn reporter<'a>(
    ///     got: &mut RescueResolved<'a, Asynchronous>,
    /// ) -> Result<AsyncStepReporter<'a>> {
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
