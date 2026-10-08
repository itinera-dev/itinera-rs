//! What cucumber keeps for one scenario while its sentences run.

/// The state of one scenario, made new for each.
#[derive(Debug, Default, cucumber::World)]
pub(crate) struct World {}
