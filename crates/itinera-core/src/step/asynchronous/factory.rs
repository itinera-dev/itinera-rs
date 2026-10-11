//! Asynchronous step factories: how an asynchronous workflow builds a step for each attempt.

use std::future::Future;

use super::AsyncStep;
use crate::error::Error;
use crate::mode::Asynchronous;
use crate::step::{Outcome, Resolved, StepNeeds};

/// What an asynchronous workflow holds to build a step for each attempt: the step declares what
/// it needs, and the factory builds it from what was resolved.
///
/// A closure that returns a future of `Result<Outcome, Error>` is a factory of a step that needs
/// nothing.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::step::{AsyncStep, AsyncStepFactory, Input, Outcome, Resolved, StepNeeds};
///
/// struct Charge {
///     amount: i64,
/// }
///
/// impl AsyncStep for Charge {
///     async fn run(self) -> Result<Outcome, Error> {
///         Ok(Outcome::success())
///     }
/// }
///
/// struct ChargeFactory;
///
/// const AMOUNT: Input<i64> = Input::new("amount");
///
/// impl AsyncStepFactory for ChargeFactory {
///     type Step<'a> = Charge;
///
///     fn needs(&self) -> StepNeeds {
///         StepNeeds::new().input(&AMOUNT)
///     }
///
///     fn build<'a>(&'a self, got: &mut Resolved<'a, Asynchronous>) -> Result<Charge, Error> {
///         Ok(Charge {
///             amount: got.input(&AMOUNT)?,
///         })
///     }
/// }
/// ```
pub trait AsyncStepFactory: Send + Sync + 'static {
    /// The step it builds, which may borrow its attempt.
    type Step<'a>: AsyncStep;

    /// What the step needs. It is asked once, when the step descriptor is made.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{AsyncStepFactory, StepNeeds};
    ///
    /// fn needs(factory: &impl AsyncStepFactory) -> StepNeeds {
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
    /// use itinera::mode::Asynchronous;
    /// use itinera::step::{AsyncStepFactory, Resolved};
    ///
    /// fn build<'a, F: AsyncStepFactory>(
    ///     factory: &'a F,
    ///     got: &mut Resolved<'a, Asynchronous>,
    /// ) -> Result<F::Step<'a>, Error> {
    ///     factory.build(got)
    /// }
    /// ```
    fn build<'a>(&'a self, got: &mut Resolved<'a, Asynchronous>) -> Result<Self::Step<'a>, Error>;
}

impl<F, R> AsyncStepFactory for F
where
    F: Fn() -> R + Send + Sync + 'static,
    R: Future<Output = Result<Outcome, Error>> + Send,
{
    type Step<'a> = &'a F;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
    }

    fn build<'a>(&'a self, _: &mut Resolved<'a, Asynchronous>) -> Result<&'a F, Error> {
        Ok(self)
    }
}
