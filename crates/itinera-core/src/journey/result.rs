//! What running a journey returns: its ID and how it ended.

mod status;
mod status_kind;

pub use status::JourneyStatus;
pub use status_kind::StatusKind;

use super::{JourneyId, Output, OutputError};

/// What running a journey returns: a business outcome, with the journey's ID and how it ended.
///
/// It holds no step statuses, attempt counts or step names: those are in the event stream. It is
/// made only by itinera's executors, and is not `Clone`, since it may hold an [`Error`].
///
/// [`Error`]: crate::error::Error
///
/// # Examples
///
/// ```
/// use itinera::journey::JourneyResult;
///
/// fn summary(result: &JourneyResult) -> String {
///     format!("{}: {}", result.journey_id, result.status.kind())
/// }
///
/// fn succeeded(result: &JourneyResult) -> bool {
///     result.status.is_succeeded()
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub struct JourneyResult {
    /// The ID of the journey that ran.
    pub journey_id: JourneyId,
    /// How the journey ended, with what explains it.
    pub status: JourneyStatus,
}

impl JourneyResult {
    /// The journey's output, read from its data bag as a type of the application's own.
    ///
    /// # Errors
    ///
    /// [`OutputError::Aborted`] when the journey was aborted, since it has no data bag, and
    /// whatever error the type's [`Output::from_data`] returns.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{DataBag, JourneyResult, Output, OutputError};
    ///
    /// struct Receipt {
    ///     receipt: String,
    /// }
    ///
    /// impl Output for Receipt {
    ///     fn from_data(data: &DataBag) -> Result<Self, OutputError> {
    ///         Ok(Self {
    ///             receipt: data.required("receipt")?,
    ///         })
    ///     }
    /// }
    ///
    /// fn receipt(result: &JourneyResult) -> Result<String, OutputError> {
    ///     Ok(result.output::<Receipt>()?.receipt)
    /// }
    /// ```
    pub fn output<T: Output>(&self) -> Result<T, OutputError> {
        self.status
            .data()
            .ok_or(OutputError::Aborted)
            .and_then(T::from_data)
    }
}
