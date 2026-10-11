//! A journey's output read as a type of the application's own.

use super::DataBag;

/// A type read from the data bag of a journey that succeeded or failed, with
/// [`JourneyResult::output`](super::JourneyResult::output).
///
/// `#[derive(Output)]` implements it, reading each field from the key it names. An application
/// may also implement it by hand, reading each key with [`DataBag::required`] or
/// [`DataBag::optional`].
///
/// # Examples
///
/// ```
/// use itinera::journey::{DataBag, Output, OutputError};
///
/// struct Receipt {
///     receipt: String,
///     note: Option<String>,
/// }
///
/// impl Output for Receipt {
///     fn from_data(data: &DataBag) -> Result<Self, OutputError> {
///         Ok(Self {
///             receipt: data.required("receipt")?,
///             note: data.optional("note")?,
///         })
///     }
/// }
/// ```
pub trait Output: Sized {
    /// Reads the output from the data bag.
    ///
    /// # Errors
    ///
    /// When a key the output requires is absent, or a key holds a value of another type.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{DataBag, Output, OutputError};
    ///
    /// fn receipt<T: Output>(data: &DataBag) -> Result<T, OutputError> {
    ///     T::from_data(data)
    /// }
    /// ```
    fn from_data(data: &DataBag) -> Result<Self, OutputError>;
}

/// Why a journey's output could not be read.
///
/// # Examples
///
/// ```
/// use itinera::journey::OutputError;
///
/// let error = OutputError::Missing {
///     key: "receipt".to_owned(),
/// };
/// assert_eq!(error.to_string(), "the data bag holds no value under `receipt`");
/// ```
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OutputError {
    /// The journey was aborted, and an aborted journey has no data bag.
    #[error("the journey was aborted, so it has no data bag to read")]
    Aborted,
    /// The output requires a key that the data bag does not hold.
    #[error("the data bag holds no value under `{key}`")]
    Missing {
        /// The key the output requires.
        key: String,
    },
    /// The data bag holds a value of another type under a key.
    #[error("the value under `{key}` is not of the type `{expected}`")]
    WrongType {
        /// The key whose value has another type.
        key: String,
        /// The type the output reads under the key.
        expected: &'static str,
    },
}
