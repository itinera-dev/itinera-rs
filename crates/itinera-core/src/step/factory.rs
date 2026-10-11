//! Step factories: how a workflow builds a step for each attempt, and how a step descriptor
//! holds a factory of either mode.

use std::future::Future;
use std::pin::Pin;

use super::{Got, Outcome, Resolved, Step, StepNeeds};
use crate::error::Error;

/// What a synchronous workflow holds to build a step for each attempt: the step declares what it
/// needs, and the factory builds it from what was resolved.
///
/// A closure that returns `Result<Outcome, Error>` is a factory of a step that needs nothing.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{Input, Outcome, Resolved, Step, StepFactory, StepNeeds};
///
/// struct Charge {
///     amount: i64,
/// }
///
/// impl Step for Charge {
///     fn run(self) -> Result<Outcome, Error> {
///         Ok(Outcome::success())
///     }
/// }
///
/// struct ChargeFactory;
///
/// const AMOUNT: Input<i64> = Input::new("amount");
///
/// impl StepFactory for ChargeFactory {
///     type Step<'a> = Charge;
///
///     fn needs(&self) -> StepNeeds {
///         StepNeeds::new().input(&AMOUNT)
///     }
///
///     fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charge, Error> {
///         Ok(Charge {
///             amount: got.input(&AMOUNT)?,
///         })
///     }
/// }
/// ```
pub trait StepFactory: Send + Sync + 'static {
    /// The step it builds, which may borrow its attempt.
    type Step<'a>: Step;

    /// What the step needs. It is asked once, when the step descriptor is made.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{StepFactory, StepNeeds};
    ///
    /// fn needs(factory: &impl StepFactory) -> StepNeeds {
    ///     factory.needs()
    /// }
    /// ```
    fn needs(&self) -> StepNeeds;

    /// Builds the step for one attempt, from what was resolved for it.
    ///
    /// # Errors
    ///
    /// An error means the step could not be built, which aborts the journey with
    /// `step could not be built`. [`Interrupted`](crate::error::Interrupted), propagated from the
    /// step's reporter, is the exception: the journey was already aborted with `reporter failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{Resolved, StepFactory};
    ///
    /// fn build<'a, F: StepFactory>(
    ///     factory: &'a F,
    ///     got: &mut Resolved<'a>,
    /// ) -> Result<F::Step<'a>, Error> {
    ///     factory.build(got)
    /// }
    /// ```
    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Self::Step<'a>, Error>;
}

impl<F> StepFactory for F
where
    F: Fn() -> Result<Outcome, Error> + Send + Sync + 'static,
{
    type Step<'a> = &'a F;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
    }

    fn build<'a>(&'a self, _: &mut Resolved<'a>) -> Result<&'a F, Error> {
        Ok(self)
    }
}

/// A step being run: its outcome, or the error that ended it abnormally, once it finishes.
pub(crate) type Running<'a> = Pin<Box<dyn Future<Output = Result<Outcome, Error>> + Send + 'a>>;

/// A step factory of either mode, as a step descriptor holds it.
pub(crate) trait Attempts: Send + Sync {
    /// Builds the step for one attempt and starts it, or fails to build it.
    fn attempt<'a>(&'a self, got: Got<'a>) -> Result<Running<'a>, Error>;
}
