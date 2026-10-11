//! Rescue steps: a step a step descriptor may hold to fix the cause of the step's failure, after
//! which the step runs once more.

use crate::error::Error;

#[cfg(feature = "async")]
mod asynchronous;
mod described;
mod factory;
mod from_requests;
mod from_rescued;
mod needs;
mod outcome;
mod resolved;

#[cfg(feature = "async")]
pub use asynchronous::{AsyncRescueFactory, AsyncRescueStep};
pub use described::DescribedRescueStep;
pub use factory::RescueFactory;
pub(crate) use factory::Rescues;
pub use from_requests::FromRescueRequests;
pub(crate) use from_rescued::FromRescued;
pub use needs::RescueNeeds;
pub use outcome::RescueOutcome;
pub use resolved::RescueResolved;

/// A rescue step: built once, after an attempt of the step it rescues failed, it fixes the cause
/// of that failure when run, and reports a [`RescueOutcome`].
///
/// It is a step in everything but its outcome, which cannot be a skip: a rescue step either fixes
/// the cause or fails. Returning an error is an abnormal termination of the rescue step.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{Reason, RescueOutcome, RescueStep};
///
/// struct RefreshToken {
///     refreshable: bool,
/// }
///
/// impl RescueStep for RefreshToken {
///     fn run(self) -> Result<RescueOutcome, Error> {
///         if self.refreshable {
///             Ok(RescueOutcome::success())
///         } else {
///             Ok(RescueOutcome::failure(Reason::new("no-refresh")))
///         }
///     }
/// }
/// ```
pub trait RescueStep {
    /// Fixes the cause of the failure and reports how it went.
    ///
    /// # Errors
    ///
    /// An error is an abnormal termination of the rescue step, which fails the journey.
    /// [`Interrupted`](crate::error::Interrupted), propagated from the step's reporter, is the
    /// exception: the journey was already aborted with `reporter failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::step::{RescueOutcome, RescueStep};
    ///
    /// fn outcome_of(rescue: impl RescueStep) -> Result<RescueOutcome, Error> {
    ///     rescue.run()
    /// }
    /// ```
    fn run(self) -> Result<RescueOutcome, Error>;
}

impl<F: Fn() -> Result<RescueOutcome, Error>> RescueStep for &F {
    fn run(self) -> Result<RescueOutcome, Error> {
        self()
    }
}
