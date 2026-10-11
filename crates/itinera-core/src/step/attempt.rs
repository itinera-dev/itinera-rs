//! A step's attempts, counted from 1.

use std::num::NonZeroU32;

use super::StepName;

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
    /// The attempt number, from 1.
    pub attempt: NonZeroU32,
}

impl StepAttempt {
    /// The first attempt of a step.
    pub(crate) fn first(step: StepName) -> Self {
        Self {
            step,
            attempt: NonZeroU32::MIN,
        }
    }

    /// The attempt after this one.
    pub(crate) fn next(&self) -> Self {
        Self {
            step: self.step,
            // A retry budget is a `u16`, so attempts never come near `u32::MAX`.
            attempt: self.attempt.saturating_add(1),
        }
    }
}
