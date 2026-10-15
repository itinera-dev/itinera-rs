//! Rescue step factories: how a synchronous workflow builds a rescue step.

use super::{RescueNeeds, RescueOutcome, RescueResolved, RescueStep};
use crate::error::Error;
use crate::step::{Attempts, Got, Outcome};

/// What a synchronous workflow holds to build a rescue step: the rescue step declares what it
/// needs, and the factory builds it from what was resolved.
///
/// A closure that returns `Result<RescueOutcome, Error>` is a factory of a rescue step that
/// needs nothing.
///
/// # Examples
///
/// ```
/// use itinera::error::Result;
/// use itinera::step::{
///     Input, Reason, RescueFactory, RescueNeeds, RescueOutcome, RescueResolved, RescueStep,
/// };
///
/// struct RefreshToken {
///     account: String,
///     reason: Reason,
/// }
///
/// impl RescueStep for RefreshToken {
///     fn run(self) -> Result<RescueOutcome> {
///         println!("refreshing {} after {}", self.account, self.reason.code());
///         Ok(RescueOutcome::success())
///     }
/// }
///
/// struct RefreshFactory;
///
/// const ACCOUNT: Input<String> = Input::new("account");
///
/// impl RescueFactory for RefreshFactory {
///     type Step<'a> = RefreshToken;
///
///     fn needs(&self) -> RescueNeeds {
///         RescueNeeds::new().from_rescued_step(&ACCOUNT).reason()
///     }
///
///     fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<RefreshToken> {
///         Ok(RefreshToken {
///             account: got.from_rescued_step(&ACCOUNT)?,
///             reason: got.reason()?,
///         })
///     }
/// }
/// ```
pub trait RescueFactory: Send + Sync + 'static {
    /// The rescue step it builds, which may borrow its attempt.
    type Step<'a>: RescueStep;

    /// What the rescue step needs. It is asked once, when the rescue step is given to a step.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{RescueFactory, RescueNeeds};
    ///
    /// fn needs(factory: &impl RescueFactory) -> RescueNeeds {
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
    /// use itinera::error::Result;
    /// use itinera::step::{RescueFactory, RescueResolved};
    ///
    /// fn build<'a, F: RescueFactory>(
    ///     factory: &'a F,
    ///     got: &mut RescueResolved<'a>,
    /// ) -> Result<F::Step<'a>> {
    ///     factory.build(got)
    /// }
    /// ```
    fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<Self::Step<'a>, Error>;
}

impl<F> RescueFactory for F
where
    F: Fn() -> Result<RescueOutcome, Error> + Send + Sync + 'static,
{
    type Step<'a> = &'a F;

    fn needs(&self) -> RescueNeeds {
        RescueNeeds::new()
    }

    fn build<'a>(&'a self, _: &mut RescueResolved<'a>) -> Result<&'a F, Error> {
        Ok(self)
    }
}

/// A synchronous rescue step factory, whose rescue steps run as soon as they are built.
#[derive(derive_more::Constructor)]
pub(crate) struct Rescues<F> {
    factory: F,
}

#[async_trait::async_trait]
impl<F: RescueFactory> Attempts for Rescues<F> {
    async fn attempt<'a>(&'a self, got: Got<'a>) -> Result<Result<Outcome, Error>, Error> {
        let rescue = self.factory.build(&mut RescueResolved::new(got))?;
        Ok(rescue.run().map(Outcome::from))
    }

    #[cfg(any(test, feature = "testing"))]
    fn build<'a>(&'a self, got: Got<'a>) -> Result<(), Error> {
        self.factory.build(&mut RescueResolved::new(got)).map(drop)
    }
}
