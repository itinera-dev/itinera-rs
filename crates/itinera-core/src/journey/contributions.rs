//! A recorder for tests, which lends a contributor and keeps what is contributed through it.

use super::{Contributor, Read};
use crate::value::Value;

/// A recorder for tests: it lends a [`Contributor`] that a test passes to a step's `run` or to a
/// hook, and keeps what is contributed through it, for the test to read once the call returns.
///
/// Nothing is committed anywhere. As in an attempt, a key contributed twice keeps its last value
/// in the place it was first contributed.
///
/// It is available with the `testing` feature, which an application enables under
/// `[dev-dependencies]`.
///
/// # Examples
///
/// ```
/// use itinera::error::{Error, Result};
/// use itinera::journey::{Contributions, Contributor, Read};
/// use itinera::step::Outcome;
///
/// struct Charge {
///     price: i64,
/// }
///
/// impl Charge {
///     fn run(self, contributor: &mut Contributor) -> Result<Outcome> {
///         contributor.contribute("charged", self.price);
///         Ok(Outcome::success())
///     }
/// }
///
/// let mut contributions = Contributions::default();
/// Charge { price: 40 }.run(contributions.contributor())?;
///
/// assert_eq!(contributions.read::<i64>("charged"), Read::Present(40));
/// # Ok::<(), Error>(())
/// ```
#[derive(Debug, Default)]
pub struct Contributions {
    contributed: Contributor,
}

impl Contributions {
    /// Lends a contributor that contributes into this recorder.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::Contributions;
    ///
    /// let mut contributions = Contributions::default();
    /// contributions.contributor().contribute("receipt", "R-1".to_string());
    /// assert_eq!(contributions.keys().collect::<Vec<_>>(), ["receipt"]);
    /// ```
    pub fn contributor(&mut self) -> &mut Contributor {
        &mut self.contributed
    }

    /// Reads the value contributed under a key as a `T`: a copy of it, or what kept it from
    /// being one.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::{Contributions, Read};
    ///
    /// let mut contributions = Contributions::default();
    /// contributions.contributor().contribute("charged", 40_i64);
    /// assert_eq!(contributions.read::<i64>("charged"), Read::Present(40));
    /// assert_eq!(contributions.read::<String>("charged"), Read::OtherType);
    /// assert_eq!(contributions.read::<i64>("refunded"), Read::Absent);
    /// ```
    #[must_use]
    pub fn read<T: Value>(&self, key: &str) -> Read<T> {
        Read::of(self.contributed.get(key))
    }

    /// The keys contributed, in the order they were first contributed.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::journey::Contributions;
    ///
    /// assert_eq!(Contributions::default().keys().count(), 0);
    /// ```
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.contributed.keys()
    }
}
