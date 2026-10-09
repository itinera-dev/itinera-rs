use std::future::Future;

use super::{Attempts, Got, Outcome, Resolved, Running, StepDescriptor, StepName, StepNeeds};
use crate::error::Error;
use crate::mode::Asynchronous;

/// A step of an asynchronous workflow: built for one attempt, it does its work when run, and
/// reports an [`Outcome`].
///
/// Everything [`Step`](super::Step) says applies to it. A step whose work is all synchronous
/// still implements this trait, with a body that never awaits.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{AsyncStep, Outcome};
///
/// struct Ship;
///
/// impl AsyncStep for Ship {
///     async fn run(self) -> Result<Outcome, Error> {
///         Ok(Outcome::success())
///     }
/// }
/// ```
pub trait AsyncStep: Send {
    /// Does the step's work and reports its outcome.
    ///
    /// # Errors
    ///
    /// An error is an abnormal termination of the attempt. It never aborts the journey.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{AsyncStep, Outcome};
    ///
    /// async fn outcome_of(step: impl AsyncStep) -> Result<Outcome, Error> {
    ///     step.run().await
    /// }
    /// ```
    fn run(self) -> impl Future<Output = Result<Outcome, Error>> + Send;
}

impl<F, R> AsyncStep for &F
where
    F: Fn() -> R + Sync,
    R: Future<Output = Result<Outcome, Error>> + Send,
{
    fn run(self) -> impl Future<Output = Result<Outcome, Error>> + Send {
        self()
    }
}

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
    /// An error means the step could not be built, which aborts the journey.
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

/// An asynchronous step factory, whose steps are awaited.
struct Asynchronously<F> {
    factory: F,
}

impl<F: AsyncStepFactory> Attempts for Asynchronously<F> {
    fn attempt<'a>(&'a self, got: Got<'a>) -> Result<Running<'a>, Error> {
        let step = self.factory.build(&mut Resolved::new(got))?;
        Ok(Box::pin(step.run()))
    }
}

impl StepDescriptor<Asynchronous> {
    /// Describes a step of an asynchronous workflow with its name and its factory, with no
    /// policies.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    ///
    /// let ship = StepDescriptor::new_async(step_name!("ship"), async || Ok(Outcome::success()));
    /// ```
    pub fn new_async(name: StepName, factory: impl AsyncStepFactory) -> Self {
        Self::holding(name, factory.needs(), Box::new(Asynchronously { factory }))
    }
}
