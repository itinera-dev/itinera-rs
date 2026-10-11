//! Asynchronous rescue steps, and how an asynchronous workflow builds them.

use std::future::Future;

mod factory;

pub use factory::AsyncRescueFactory;

use super::RescueOutcome;
use crate::error::Error;

/// A rescue step of an asynchronous workflow: built once, after an attempt of the step it rescues
/// failed, it fixes the cause of that failure when run, and reports a [`RescueOutcome`].
///
/// Everything [`RescueStep`](super::RescueStep) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{AsyncRescueStep, RescueOutcome};
///
/// struct RefreshToken;
///
/// impl AsyncRescueStep for RefreshToken {
///     async fn run(self) -> Result<RescueOutcome, Error> {
///         Ok(RescueOutcome::success())
///     }
/// }
/// ```
pub trait AsyncRescueStep: Send {
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
    /// use itinera::step::{AsyncRescueStep, RescueOutcome};
    ///
    /// async fn outcome_of(rescue: impl AsyncRescueStep) -> Result<RescueOutcome, Error> {
    ///     rescue.run().await
    /// }
    /// ```
    fn run(self) -> impl Future<Output = Result<RescueOutcome, Error>> + Send;
}

impl<F, R> AsyncRescueStep for &F
where
    F: Fn() -> R + Sync,
    R: Future<Output = Result<RescueOutcome, Error>> + Send,
{
    fn run(self) -> impl Future<Output = Result<RescueOutcome, Error>> + Send {
        self()
    }
}
