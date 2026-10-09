//! The error of custom code.

use std::fmt;

/// The error every piece of custom code returns when it fails: steps, hooks, role operations,
/// policy factories, reporters, dispatchers and dispatcher factories.
///
/// Any `std::error::Error + Send + Sync + 'static` converts into it, so `?` works on the errors
/// of any library. Because of that conversion it does not implement `std::error::Error` itself;
/// it offers [`Display`](fmt::Display), [`source`](Error::source) and [`Error::msg`] instead, and
/// converts into `Box<dyn std::error::Error + Send + Sync>`.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
///
/// fn parse(text: &str) -> Result<i64, Error> {
///     Ok(text.parse::<i64>()?)
/// }
///
/// assert_eq!(parse("42").ok(), Some(42));
/// assert!(parse("forty-two").is_err());
/// ```
#[derive(derive_more::Debug, derive_more::Display)]
#[debug("{inner:?}")]
pub struct Error {
    inner: Box<dyn std::error::Error + Send + Sync + 'static>,
}

impl Error {
    /// Makes an error from a message alone.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    ///
    /// let error = Error::msg("the ledger is closed");
    /// assert_eq!(error.to_string(), "the ledger is closed");
    /// ```
    pub fn msg<M>(message: M) -> Self
    where
        M: fmt::Display + fmt::Debug + Send + Sync + 'static,
    {
        Self {
            inner: Box::new(Message { message }),
        }
    }

    /// The error that caused this one, if the original error named one.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fmt;
    /// use itinera::error::Error;
    ///
    /// #[derive(Debug)]
    /// struct Outer {
    ///     cause: std::num::ParseIntError,
    /// }
    ///
    /// impl fmt::Display for Outer {
    ///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    ///         f.write_str("the amount could not be read")
    ///     }
    /// }
    ///
    /// impl std::error::Error for Outer {
    ///     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    ///         Some(&self.cause)
    ///     }
    /// }
    ///
    /// let cause = "x".parse::<i64>().unwrap_err();
    /// let error = Error::from(Outer {
    ///     cause: cause.clone(),
    /// });
    /// assert_eq!(error.source().map(|s| s.to_string()), Some(cause.to_string()));
    /// assert!(Error::msg("no cause").source().is_none());
    /// ```
    pub fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source()
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(error: E) -> Self {
        Self {
            inner: Box::new(error),
        }
    }
}

impl From<Error> for Box<dyn std::error::Error + Send + Sync + 'static> {
    fn from(error: Error) -> Self {
        error.inner
    }
}

/// The executor's signal that ends a step or hook at an emit call: a reporter failed while the
/// event was delivered, and the journey is aborted with `reporter failed`.
///
/// The step or hook propagates it with `?`, which converts it into an [`Error`]. The abort stands
/// whatever it does afterwards: nothing more it emits is delivered, and its outcome and
/// contributions are ignored. Only itinera makes one.
///
/// # Examples
///
/// ```
/// use itinera::error::{Error, Interrupted};
///
/// fn propagated(emitted: Result<(), Interrupted>) -> Result<(), Error> {
///     emitted?;
///     Ok(())
/// }
/// ```
#[derive(Debug, thiserror::Error)]
#[error("the journey was aborted: a reporter failed while the event was delivered")]
#[non_exhaustive]
pub struct Interrupted;

#[derive(derive_more::Debug, derive_more::Display)]
#[debug("{message:?}")]
struct Message<M> {
    message: M,
}

impl<M: fmt::Display + fmt::Debug> std::error::Error for Message<M> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_shows_the_message_of_the_error_it_was_made_from() {
        let error = Error::from("x".parse::<i64>().unwrap_err());
        assert_eq!(error.to_string(), "invalid digit found in string");
    }

    #[test]
    fn an_error_converts_back_into_a_boxed_standard_error() {
        let boxed: Box<dyn std::error::Error + Send + Sync> = Error::msg("closed").into();
        assert_eq!(boxed.to_string(), "closed");
    }
}
