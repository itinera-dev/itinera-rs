//! What cucumber keeps for one scenario while its sentences run.

use crate::declaration::Admission;
use crate::executor::Executor;
use crate::journey::Journey;
use crate::model::Model;
use crate::record::Recorders;
use crate::step::StepsRun;

/// The state of one scenario, made new for each.
#[derive(Debug, Default, cucumber::World)]
pub(crate) struct World {
    pub(crate) model: Model,
    /// The executor this copy of the scenario runs under, from its tag.
    pub(crate) executor: Option<Executor>,
    /// What the reporters of the scenario's journeys received.
    pub(crate) recorders: Recorders,
    /// How many times the scenario's steps ran.
    pub(crate) steps_run: StepsRun,
    /// What building the scenario's workflow gave, once it was admitted or listed.
    pub(crate) admission: Option<Admission>,
    /// The journeys the scenario ran, in order.
    pub(crate) journeys: Vec<Journey>,
}
