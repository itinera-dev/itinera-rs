//! Why an instance could not be created.

use crate::error::Error;

/// Why an instance could not be created. No journey exists yet, so nothing is reported.
///
/// # Examples
///
/// ```
/// use itinera::instance::InstanceError;
///
/// fn describe(error: &InstanceError) -> String {
///     error.to_string()
/// }
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum InstanceError {
    /// The workflow's generator failed to produce the journey ID.
    #[error("the journey ID could not be produced: {0}")]
    JourneyId(Error),
}
