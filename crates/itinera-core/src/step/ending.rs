//! How a scripted attempt ends.

use super::{Outcome, Reason};
use crate::error::Error;

/// How a scripted attempt ends: what its step's `run` would return, played in its place. A
/// test's scripted run plays one for each attempt its script describes, and a test tool that
/// writes real steps from a script can end them the same way.
///
/// It is available with the `testing` feature, which an application enables under
/// `[dev-dependencies]`.
///
/// # Examples
///
/// ```
/// use itinera::step::{Ending, Reason};
///
/// let declined = Ending::Failure(Reason::new("declined")).outcome()?;
/// assert!(declined.is_failure());
/// assert_eq!(declined.reason().map(Reason::code), Some("declined"));
/// # Ok::<(), itinera::error::Error>(())
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Ending {
    /// The attempt succeeds.
    Success,
    /// The attempt fails for this reason, and suggests no retry.
    Failure(Reason),
    /// The attempt fails for this reason, and suggests retrying the step.
    RetriableFailure(Reason),
    /// The attempt has nothing to do, for this reason if one is given, and the step is skipped.
    Skipped(Option<Reason>),
    /// The attempt terminates abnormally, with an error carrying this message.
    AbnormalTermination(String),
}

impl Ending {
    /// What a step's `run` returns to end this way.
    ///
    /// # Errors
    ///
    /// An error carrying the message, for an abnormal termination.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Ending;
    ///
    /// assert!(Ending::Success.outcome()?.is_success());
    /// let crashed = Ending::AbnormalTermination("gateway down".to_owned()).outcome();
    /// assert_eq!(crashed.err().map(|error| error.to_string()).as_deref(), Some("gateway down"));
    /// # Ok::<(), itinera::error::Error>(())
    /// ```
    pub fn outcome(&self) -> Result<Outcome, Error> {
        match self {
            Self::Success => Ok(Outcome::success()),
            Self::Failure(reason) => Ok(Outcome::failure(reason.clone())),
            Self::RetriableFailure(reason) => Ok(Outcome::retriable_failure(reason.clone())),
            Self::Skipped(None) => Ok(Outcome::skipped()),
            Self::Skipped(Some(reason)) => Ok(Outcome::skipped_because(reason.clone())),
            Self::AbnormalTermination(message) => Err(Error::msg(message.clone())),
        }
    }
}
