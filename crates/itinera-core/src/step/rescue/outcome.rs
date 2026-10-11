//! What a rescue step reports when it runs to the end.

use crate::step::{Outcome, Reason};

/// What a rescue step reports when it runs to the end: Success, Failure or a retriable Failure.
///
/// It is an [`Outcome`] that cannot be Skipped, and converts into one.
///
/// # Examples
///
/// ```
/// use itinera::step::{Reason, RescueOutcome};
///
/// fn refreshed(token: Option<&str>) -> RescueOutcome {
///     match token {
///         Some(_) => RescueOutcome::success(),
///         None => RescueOutcome::failure(Reason::new("no-refresh")),
///     }
/// }
/// ```
#[derive(Debug, derive_more::Into)]
pub struct RescueOutcome {
    outcome: Outcome,
}

impl RescueOutcome {
    /// The rescue step fixed the cause: the step it rescues runs again.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::RescueOutcome;
    ///
    /// let fixed = RescueOutcome::success();
    /// ```
    pub fn success() -> Self {
        Self {
            outcome: Outcome::success(),
        }
    }

    /// The rescue step could not fix the cause, and trying again would not help.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Reason, RescueOutcome};
    ///
    /// let stuck = RescueOutcome::failure(Reason::new("no-refresh"));
    /// ```
    pub fn failure(reason: Reason) -> Self {
        Self {
            outcome: Outcome::failure(reason),
        }
    }

    /// The rescue step could not fix the cause, and trying again might help. A rescue step is
    /// never attempted again, so this fails the journey with `retries exhausted`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Reason, RescueOutcome};
    ///
    /// let busy = RescueOutcome::retriable_failure(Reason::new("token-service-busy"));
    /// ```
    pub fn retriable_failure(reason: Reason) -> Self {
        Self {
            outcome: Outcome::retriable_failure(reason),
        }
    }
}
