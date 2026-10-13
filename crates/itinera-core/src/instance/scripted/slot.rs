//! Which attempts of a step an attempt's script describes.

/// Which attempts of a step an attempt's script describes.
#[derive(Clone, Copy, Debug)]
pub(super) enum Slot {
    /// The attempt after the last numbered one.
    Next,
    /// Every attempt after the numbered ones, which is every attempt when none is numbered.
    Remaining,
}
