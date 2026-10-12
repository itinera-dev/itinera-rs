//! What committing a contribution did to the data bag.

/// What committing a contribution did to the data bag.
///
/// # Examples
///
/// ```
/// use itinera::instance::Committed;
///
/// fn overwrote(committed: Committed) -> bool {
///     committed == Committed::Overwritten
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Committed {
    /// The data bag had no value under the key.
    Added,
    /// The value replaced the one the data bag had under the key.
    Overwritten,
}
