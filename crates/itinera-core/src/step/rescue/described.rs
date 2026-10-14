//! Rescue step types that know how a step descriptor holds them, and the shorthands for their
//! outcomes.

use super::RescueOutcome;
use crate::error::Error;
use crate::mode::Mode;
use crate::step::{Reason, StepDescriptor};

/// A rescue step type that knows how a step descriptor holds it, so that a step can be given it
/// by its type, with [`StepDescriptor::rescued_by`].
///
/// `#[rescue_step]` implements it. Every rescue step type implementing it also has the
/// shorthands for its outcomes, such as `Self::succeed()` and `Self::fail("no-refresh")`, and has
/// no shorthand for a skip, since a rescue step cannot skip.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Synchronous;
/// use itinera::step::{
///     DescribedRescueStep, Outcome, RescueOutcome, RescueStep, StepDescriptor, step_name,
/// };
///
/// struct RefreshToken;
///
/// impl RescueStep for RefreshToken {
///     fn run(self) -> Result<RescueOutcome, Error> {
///         Self::succeed()
///     }
/// }
///
/// impl DescribedRescueStep for RefreshToken {
///     type Mode = Synchronous;
///
///     fn rescue<W>(step: StepDescriptor<W>) -> StepDescriptor<W> {
///         step.rescue(|| RefreshToken.run())
///     }
/// }
///
/// struct Orders;
///
/// let charge = StepDescriptor::<Orders>::new(step_name!("charge"), || Ok(Outcome::success()))
///     .rescued_by::<RefreshToken>();
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a rescue step a step can be given by its type",
    label = "not a described rescue step",
    note = "write `#[rescue_step]` on the block that holds its `run`"
)]
pub trait DescribedRescueStep {
    /// The mode of the workflows whose steps it can rescue: an asynchronous rescue step only
    /// rescues a step of an asynchronous workflow.
    type Mode: Mode;

    /// The keys of the services the rescue step requests, in the order it declares them: none
    /// unless an implementation says otherwise. `#[rescue_step]` lists them, so that
    /// `#[workflow]` refuses to compile a workflow that provides no service for one of them; the
    /// descriptor's `build()` checks every rescue step either way.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Synchronous;
    /// use itinera::service::Service;
    /// use itinera::step::{
    ///     DescribedRescueStep, RescueFactory, RescueNeeds, RescueOutcome, RescueResolved,
    ///     RescueStep, StepDescriptor,
    /// };
    ///
    /// trait TokenService: Send + Sync {}
    ///
    /// const TOKENS: Service<dyn TokenService> = Service::new("tokens");
    ///
    /// struct Refresh<'a> {
    ///     tokens: &'a dyn TokenService,
    /// }
    ///
    /// impl RescueStep for Refresh<'_> {
    ///     fn run(self) -> Result<RescueOutcome, Error> {
    ///         Ok(RescueOutcome::success())
    ///     }
    /// }
    ///
    /// struct Refreshing;
    ///
    /// impl RescueFactory for Refreshing {
    ///     type Step<'a> = Refresh<'a>;
    ///
    ///     fn needs(&self) -> RescueNeeds {
    ///         RescueNeeds::new().service(&TOKENS)
    ///     }
    ///
    ///     fn build<'a>(&'a self, got: &mut RescueResolved<'a>) -> Result<Refresh<'a>, Error> {
    ///         Ok(Refresh {
    ///             tokens: got.service(&TOKENS)?,
    ///         })
    ///     }
    /// }
    ///
    /// struct RefreshToken;
    ///
    /// impl DescribedRescueStep for RefreshToken {
    ///     type Mode = Synchronous;
    ///
    ///     const SERVICES: &'static [&'static str] = &["tokens"];
    ///
    ///     fn rescue<W>(step: StepDescriptor<W>) -> StepDescriptor<W> {
    ///         step.rescue(Refreshing)
    ///     }
    /// }
    ///
    /// assert_eq!(RefreshToken::SERVICES, ["tokens"]);
    /// ```
    const SERVICES: &'static [&'static str] = &[];

    /// Gives `step` this rescue step, replacing any it had.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{DescribedRescueStep, StepDescriptor};
    ///
    /// fn rescued<R: DescribedRescueStep, W>(
    ///     step: StepDescriptor<W, R::Mode>,
    /// ) -> StepDescriptor<W, R::Mode> {
    ///     R::rescue(step)
    /// }
    /// ```
    fn rescue<W>(step: StepDescriptor<W, Self::Mode>) -> StepDescriptor<W, Self::Mode>;

    /// The rescue step fixed the cause: [`RescueOutcome::success`].
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that a rescue step's `run` can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{DescribedRescueStep, RescueOutcome};
    ///
    /// fn fixed<R: DescribedRescueStep>() -> Result<RescueOutcome, Error> {
    ///     R::succeed()
    /// }
    /// ```
    fn succeed() -> Result<RescueOutcome, Error> {
        Ok(RescueOutcome::success())
    }

    /// The rescue step could not fix the cause, for this reason: [`RescueOutcome::failure`]. A
    /// rescue step is never attempted again, so this fails the journey. A text is the reason's
    /// code.
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that a rescue step's `run` can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{DescribedRescueStep, RescueOutcome};
    ///
    /// fn stuck<R: DescribedRescueStep>() -> Result<RescueOutcome, Error> {
    ///     R::fail("no-refresh")
    /// }
    /// ```
    fn fail(reason: impl Into<Reason>) -> Result<RescueOutcome, Error> {
        Ok(RescueOutcome::failure(reason.into()))
    }
}
