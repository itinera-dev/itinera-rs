//! What the failed attempt of a rescued step left for its rescue step.

use crate::error::Error;
use crate::step::{Reason, Slots};

/// What the failed attempt of the step a rescue step rescues left for the rescue step's
/// requests, as the executor resolved them.
#[derive(Debug, Default)]
pub(crate) struct FromRescued {
    pub(crate) contributed: Slots,
    /// The reason, if requested: present or absent.
    pub(crate) reason: Option<Option<Reason>>,
    /// The error, if requested: present or absent.
    pub(crate) error: Option<Option<Error>>,
}
