//! What cucumber keeps for one scenario while its sentences run.

use crate::model::Model;

/// The state of one scenario, made new for each.
#[derive(Debug, Default, cucumber::World)]
pub(crate) struct World {
    pub(crate) model: Model,
}
