//! What cucumber keeps for one scenario while its sentences run.

use crate::model::Model;
use crate::record::Recorders;

/// The state of one scenario, made new for each.
#[derive(Debug, Default, cucumber::World)]
pub(crate) struct World {
    pub(crate) model: Model,
    /// What the reporters of the scenario's journeys received.
    pub(crate) recorders: Recorders,
}
