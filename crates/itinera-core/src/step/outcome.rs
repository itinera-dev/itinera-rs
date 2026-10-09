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
}
