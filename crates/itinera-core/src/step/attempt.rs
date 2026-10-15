//! A step's attempts, counted from 1.

use std::num::NonZeroU32;

use super::StepName;

/// The number of one attempt of a step, counted from 1.
///
/// It displays as its number, and converts into a `NonZeroU32`.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroU32;
///
/// use itinera::step::Attempt;
///
/// let first = Attempt::FIRST;
/// assert!(first.is_first());
/// assert_eq!(first.to_string(), "1");
/// assert_eq!(NonZeroU32::from(first).get(), 1);
/// ```
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::Display,
    derive_more::Into,
)]
#[display("{number}")]
pub struct Attempt {
    number: NonZeroU32,
}

impl Attempt {
    /// The first attempt of a step.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Attempt;
    ///
    /// assert_eq!(Attempt::FIRST.to_string(), "1");
    /// ```
    pub const FIRST: Self = Self {
        number: NonZeroU32::MIN,
    };

    /// The attempt numbered `number`.
    #[cfg(any(test, feature = "testing"))]
    pub(crate) fn new(number: NonZeroU32) -> Self {
        Self { number }
    }

    /// Whether it is the first attempt of its step.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::Attempt;
    ///
    /// fn retried(attempt: Attempt) -> bool {
    ///     !attempt.is_first()
    /// }
    ///
    /// assert!(!retried(Attempt::FIRST));
    /// ```
    #[must_use]
    pub fn is_first(self) -> bool {
        self == Self::FIRST
    }

    /// The attempt after this one.
    pub(crate) fn next(self) -> Self {
        Self {
            // A retry budget is a `u16`, so attempts never come near `u32::MAX`.
            number: self.number.saturating_add(1),
        }
    }
}

/// A step and one of its attempts, counted from 1.
///
/// # Examples
///
/// ```
/// use itinera::step::StepAttempt;
///
/// fn describe(attempt: &StepAttempt) -> String {
///     format!("{}, attempt {}", attempt.step, attempt.attempt)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct StepAttempt {
    /// The step this attempt belongs to.
    pub step: StepName,
    /// The attempt.
    pub attempt: Attempt,
}

impl StepAttempt {
    /// The first attempt of a step.
    pub(crate) fn first(step: StepName) -> Self {
        Self {
            step,
            attempt: Attempt::FIRST,
        }
    }

    /// The attempt after this one.
    pub(crate) fn next(&self) -> Self {
        Self {
            step: self.step,
            attempt: self.attempt.next(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_first_attempt_is_first() {
        assert!(Attempt::FIRST.is_first());
        assert!(!Attempt::FIRST.next().is_first());
    }

    #[test]
    fn the_attempt_after_another_is_numbered_one_more() {
        assert_eq!(Attempt::FIRST.next().to_string(), "2");
    }
}
