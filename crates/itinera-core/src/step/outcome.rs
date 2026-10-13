use super::Reason;

/// What a step reports when it runs to the end: Success, Failure or Skipped.
///
/// An outcome says what happened, never what to do next. A failure is not retriable unless the
/// step says it is, and the retriable flag is only a suggestion: whether the step is tried again
/// depends on its retry budget and its hooks.
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
///         Outcome::failure(Reason::new("insufficient-funds"))
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
    RetriableFailure(Reason),
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

    /// The step could not complete, and trying again would not help.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, Reason};
    ///
    /// let declined = Outcome::failure(Reason::new("card-declined"));
    /// ```
    pub fn failure(reason: Reason) -> Self {
        Self {
            kind: OutcomeKind::Failure(reason),
        }
    }

    /// The step could not complete, and trying again might help.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, Reason};
    ///
    /// let unreachable = Outcome::retriable_failure(Reason::new("gateway-unreachable"));
    /// ```
    pub fn retriable_failure(reason: Reason) -> Self {
        Self {
            kind: OutcomeKind::RetriableFailure(reason),
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

    /// The step decided not to do its work, for this reason.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, Reason};
    ///
    /// let paid = Outcome::skipped_because(Reason::new("already-paid"));
    /// ```
    pub fn skipped_because(reason: Reason) -> Self {
        Self {
            kind: OutcomeKind::Skipped(Some(reason)),
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

    /// Whether the step failed, retriably or not.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, Reason};
    ///
    /// assert!(Outcome::failure(Reason::new("declined")).is_failure());
    /// assert!(Outcome::retriable_failure(Reason::new("busy")).is_failure());
    /// ```
    #[must_use]
    pub fn is_failure(&self) -> bool {
        matches!(
            self.kind,
            OutcomeKind::Failure(_) | OutcomeKind::RetriableFailure(_)
        )
    }

    /// Whether the step failed and suggests it may be tried again.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, Reason};
    ///
    /// assert!(Outcome::retriable_failure(Reason::new("busy")).is_retriable());
    /// assert!(!Outcome::failure(Reason::new("declined")).is_retriable());
    /// ```
    #[must_use]
    pub fn is_retriable(&self) -> bool {
        matches!(self.kind, OutcomeKind::RetriableFailure(_))
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
    /// let declined = Outcome::failure(Reason::new("declined"));
    /// assert_eq!(declined.reason().map(Reason::code), Some("declined"));
    /// assert!(Outcome::success().reason().is_none());
    /// ```
    #[must_use]
    pub fn reason(&self) -> Option<&Reason> {
        match &self.kind {
            OutcomeKind::Failure(reason) | OutcomeKind::RetriableFailure(reason) => Some(reason),
            OutcomeKind::Skipped(reason) => reason.as_ref(),
            OutcomeKind::Success => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// What an outcome says it is: success, failure, retriable and skipped, with its reason's
    /// code.
    fn said(outcome: &Outcome) -> ([bool; 4], Option<&str>) {
        let kinds = [
            outcome.is_success(),
            outcome.is_failure(),
            outcome.is_retriable(),
            outcome.is_skipped(),
        ];
        (kinds, outcome.reason().map(Reason::code))
    }

    #[rstest]
    #[case::a_success(Outcome::success(), [true, false, false, false], None)]
    #[case::a_failure(
        Outcome::failure(Reason::new("declined")),
        [false, true, false, false],
        Some("declined")
    )]
    #[case::a_retriable_failure(
        Outcome::retriable_failure(Reason::new("busy")),
        [false, true, true, false],
        Some("busy")
    )]
    #[case::a_skip(Outcome::skipped(), [false, false, false, true], None)]
    #[case::a_skip_with_a_reason(
        Outcome::skipped_because(Reason::new("already-paid")),
        [false, false, false, true],
        Some("already-paid")
    )]
    fn an_outcome_says_what_it_is_and_why(
        #[case] outcome: Outcome,
        #[case] kinds: [bool; 4],
        #[case] code: Option<&str>,
    ) {
        assert_eq!(said(&outcome), (kinds, code));
    }
}
