use super::Reason;

/// What a step reports when it runs to the end: Success, Failure or Skipped.
///
/// An outcome says what happened, never what to do next: whether a step that failed is tried
/// again depends only on its retry budget and its hooks.
///
/// # Examples
///
/// ```
/// use itinera::step::{Outcome, Reason};
///
/// fn charge(balance: i64, amount: i64) -> Outcome {
///     if amount == 0 {
///         Outcome::skipped()
///     } else if balance >= amount {
///         Outcome::success()
///     } else {
///         Outcome::failure("insufficient-funds")
///     }
/// }
/// ```
#[derive(Debug, derive_more::Into)]
pub struct Outcome {
    kind: OutcomeKind,
}

/// The outcomes, as the engine tells them apart.
#[derive(Debug)]
pub(crate) enum OutcomeKind {
    Success,
    Failure(Reason),
    Skipped(Option<Reason>),
}

impl Outcome {
    /// The step completed.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Outcome;
    ///
    /// fn ship() -> Outcome {
    ///     Outcome::success()
    /// }
    /// ```
    pub fn success() -> Self {
        Self {
            kind: OutcomeKind::Success,
        }
    }

    /// The step could not complete, for this reason. A text is the reason's code.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, Reason};
    ///
    /// let declined = Outcome::failure("card-declined");
    /// let expired = Outcome::failure(Reason::new("card-expired").with_message("expired in May"));
    /// ```
    pub fn failure(reason: impl Into<Reason>) -> Self {
        Self {
            kind: OutcomeKind::Failure(reason.into()),
        }
    }

    /// The step decided, from what it knows of its domain, not to do its work.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Outcome;
    ///
    /// let nothing_to_do = Outcome::skipped();
    /// ```
    pub fn skipped() -> Self {
        Self {
            kind: OutcomeKind::Skipped(None),
        }
    }

    /// The step decided not to do its work, for this reason. A text is the reason's code.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Outcome;
    ///
    /// let paid = Outcome::skipped_because("already-paid");
    /// ```
    pub fn skipped_because(reason: impl Into<Reason>) -> Self {
        Self {
            kind: OutcomeKind::Skipped(Some(reason.into())),
        }
    }

    /// Whether the step completed.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Outcome;
    ///
    /// assert!(Outcome::success().is_success());
    /// assert!(!Outcome::skipped().is_success());
    /// ```
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self.kind, OutcomeKind::Success)
    }

    /// Whether the step failed.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, Reason};
    ///
    /// assert!(Outcome::failure("declined").is_failure());
    /// assert!(!Outcome::skipped().is_failure());
    /// ```
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(self.kind, OutcomeKind::Failure(_))
    }

    /// Whether the step decided not to do its work.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Outcome;
    ///
    /// assert!(Outcome::skipped().is_skipped());
    /// ```
    #[must_use]
    pub fn is_skipped(&self) -> bool {
        matches!(self.kind, OutcomeKind::Skipped(_))
    }

    /// The reason of a failure, or of a skip that was given one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, Reason};
    ///
    /// let declined = Outcome::failure("declined");
    /// assert_eq!(declined.reason().map(Reason::code), Some("declined"));
    /// assert!(Outcome::success().reason().is_none());
    /// ```
    #[must_use]
    pub fn reason(&self) -> Option<&Reason> {
        match &self.kind {
            OutcomeKind::Failure(reason) => Some(reason),
            OutcomeKind::Skipped(reason) => reason.as_ref(),
            OutcomeKind::Success => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// What an outcome says it is: success, failure and skipped, with its reason's code.
    fn said(outcome: &Outcome) -> ([bool; 3], Option<&str>) {
        let kinds = [
            outcome.is_success(),
            outcome.is_failure(),
            outcome.is_skipped(),
        ];
        (kinds, outcome.reason().map(Reason::code))
    }

    #[rstest]
    #[case::a_success(Outcome::success(), [true, false, false], None)]
    #[case::a_failure(
        Outcome::failure("declined"),
        [false, true, false],
        Some("declined")
    )]
    #[case::a_skip(Outcome::skipped(), [false, false, true], None)]
    #[case::a_skip_with_a_reason(
        Outcome::skipped_because("already-paid"),
        [false, false, true],
        Some("already-paid")
    )]
    fn an_outcome_says_what_it_is_and_why(
        #[case] outcome: Outcome,
        #[case] kinds: [bool; 3],
        #[case] code: Option<&str>,
    ) {
        assert_eq!(said(&outcome), (kinds, code));
    }
}
