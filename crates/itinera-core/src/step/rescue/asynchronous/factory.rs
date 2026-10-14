//! Asynchronous rescue step factories: how an asynchronous workflow builds a rescue step.

use std::future::Future;

use super::AsyncRescueStep;
use crate::error::Error;
use crate::mode::Asynchronous;
use crate::step::{
    Attempts, Got, Outcome, RescueNeeds, RescueOutcome, RescueResolved, StepDescriptor,
};

/// What an asynchronous workflow holds to build a rescue step: the rescue step declares what it
/// needs, and the factory builds it from what was resolved.
///
/// A closure that returns a future of `Result<RescueOutcome, Error>` is a factory of a rescue
/// step that needs nothing.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::step::{
///     AsyncRescueFactory, AsyncRescueStep, RescueNeeds, RescueOutcome, RescueResolved,
/// };
///
/// struct RefreshToken {
///     message: Option<String>,
/// }
///
/// impl AsyncRescueStep for RefreshToken {
///     async fn run(self) -> Result<RescueOutcome, Error> {
///         println!("refreshing after {:?}", self.message);
///         Ok(RescueOutcome::success())
///     }
/// }
///
/// struct RefreshFactory;
///
/// impl AsyncRescueFactory for RefreshFactory {
///     type Step<'a> = RefreshToken;
///
///     fn needs(&self) -> RescueNeeds {
///         RescueNeeds::new().optional_error()
///     }
///
///     fn build<'a>(
///         &'a self,
///         got: &mut RescueResolved<'a, Asynchronous>,
///     ) -> Result<RefreshToken, Error> {
///         Ok(RefreshToken {
///             message: got.optional_error()?.map(|error| error.to_string()),
///         })
///     }
/// }
/// ```
pub trait AsyncRescueFactory: Send + Sync + 'static {
    /// The rescue step it builds, which may borrow its attempt.
    type Step<'a>: AsyncRescueStep;

    /// What the rescue step needs. It is asked once, when the rescue step is given to a step.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{AsyncRescueFactory, RescueNeeds};
    ///
    /// fn needs(factory: &impl AsyncRescueFactory) -> RescueNeeds {
    ///     factory.needs()
    /// }
    /// ```
    fn needs(&self) -> RescueNeeds;

    /// Builds the rescue step, from what was resolved for it.
    ///
    /// # Errors
    ///
    /// An error means the rescue step could not be built, which aborts the journey with
    /// `step could not be built`. [`Interrupted`](crate::error::Interrupted), propagated from the
    /// step's reporter, is the exception: the journey was already aborted with
    /// `reporter failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::step::{AsyncRescueFactory, RescueResolved};
    ///
    /// fn build<'a, F: AsyncRescueFactory>(
    ///     factory: &'a F,
    ///     got: &mut RescueResolved<'a, Asynchronous>,
    /// ) -> Result<F::Step<'a>, Error> {
    ///     factory.build(got)
    /// }
    /// ```
    fn build<'a>(
        &'a self,
        got: &mut RescueResolved<'a, Asynchronous>,
    ) -> Result<Self::Step<'a>, Error>;
}

impl<F, R> AsyncRescueFactory for F
where
    F: Fn() -> R + Send + Sync + 'static,
    R: Future<Output = Result<RescueOutcome, Error>> + Send,
{
    type Step<'a> = &'a F;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new()
    }

    fn build<'a>(&'a self, _: &mut RescueResolved<'a, Asynchronous>) -> Result<&'a F, Error> {
        Ok(self)
    }
}

impl<W> StepDescriptor<W, Asynchronous> {
    /// Gives the step of an asynchronous workflow a rescue step, built by the factory, replacing
    /// any it had. The rescue step's name is the step's name followed by `_rescue`.
    ///
    /// When an attempt of the step fails, the rescue step runs, and once it succeeds the step runs
    /// once more. A step with a rescue step is never retried: a workflow holding one with a retry
    /// budget is refused with `rescued step retried`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, RescueOutcome, StepDescriptor, step_name};
    ///
    /// struct Orders;
    ///
    /// let charge = StepDescriptor::<Orders, _>::new_async(step_name!("charge"), async || {
    ///     Ok(Outcome::success())
    /// })
    /// .rescue_async(async || Ok(RescueOutcome::success()));
    /// ```
    pub fn rescue_async(self, factory: impl AsyncRescueFactory) -> Self {
        let needs = factory.needs().into_step_needs();
        self.rescued(needs, Box::new(AsyncRescues::new(factory)))
    }
}

/// An asynchronous rescue step factory, whose rescue steps are awaited.
#[derive(derive_more::Constructor)]
pub(crate) struct AsyncRescues<F> {
    factory: F,
}

#[async_trait::async_trait]
impl<F: AsyncRescueFactory> Attempts for AsyncRescues<F> {
    async fn attempt<'a>(&'a self, got: Got<'a>) -> Result<Result<Outcome, Error>, Error> {
        let rescue = self.factory.build(&mut RescueResolved::new(got))?;
        Ok(rescue.run().await.map(Outcome::from))
    }

    #[cfg(any(test, feature = "testing"))]
    fn build<'a>(&'a self, got: Got<'a>) -> Result<(), Error> {
        self.factory.build(&mut RescueResolved::new(got)).map(drop)
    }
}
