//! What a rescue step reports when it runs to the end.

use crate::step::{Outcome, Reason};

/// What a rescue step reports when it runs to the end: Success or Failure.
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
///         None => RescueOutcome::failure("no-refresh"),
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

    /// The rescue step could not fix the cause, for this reason. A rescue step is never attempted
    /// again, so this fails the journey. A text is the reason's code.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::RescueOutcome;
    ///
    /// let stuck = RescueOutcome::failure("no-refresh");
    /// ```
    pub fn failure(reason: impl Into<Reason>) -> Self {
        Self {
            outcome: Outcome::failure(reason),
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

    /// Whether the rescue step could not fix the cause.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Reason, RescueOutcome};
    ///
    /// assert!(RescueOutcome::failure("no-account").is_failure());
    /// ```
    #[must_use]
    pub fn is_failure(&self) -> bool {
        self.outcome.is_failure()
    }

    /// The reason of a failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Reason, RescueOutcome};
    ///
    /// let busy = RescueOutcome::failure("busy");
    /// assert_eq!(busy.reason().map(Reason::code), Some("busy"));
    /// ```
    #[must_use]
    pub fn reason(&self) -> Option<&Reason> {
        self.outcome.reason()
    }
}
