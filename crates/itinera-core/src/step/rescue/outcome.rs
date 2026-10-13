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

    /// Whether the rescue step fixed the cause.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::RescueOutcome;
    ///
    /// assert!(RescueOutcome::success().is_success());
    /// ```
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.outcome.is_success()
    }

    /// Whether the rescue step could not fix the cause, retriably or not.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Reason, RescueOutcome};
    ///
    /// assert!(RescueOutcome::failure(Reason::new("no-account")).is_failure());
    /// ```
    #[must_use]
    pub fn is_failure(&self) -> bool {
        self.outcome.is_failure()
    }

    /// Whether the rescue step failed and suggests it may be tried again.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Reason, RescueOutcome};
    ///
    /// assert!(RescueOutcome::retriable_failure(Reason::new("busy")).is_retriable());
    /// ```
    #[must_use]
    pub fn is_retriable(&self) -> bool {
        self.outcome.is_retriable()
    }

    /// The reason of a failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Reason, RescueOutcome};
    ///
    /// let busy = RescueOutcome::retriable_failure(Reason::new("busy"));
    /// assert_eq!(busy.reason().map(Reason::code), Some("busy"));
    /// ```
    #[must_use]
    pub fn reason(&self) -> Option<&Reason> {
        self.outcome.reason()
    }
}
