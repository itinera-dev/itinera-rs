//! What cucumber keeps for one scenario while its sentences run.

use itinera::workflow::{Violations, WorkflowDescriptor};

use crate::declaration::{ScriptedWorkflow, StepsRun};
use crate::model::Model;
use crate::record::Recorders;

/// The state of one scenario, made new for each.
#[derive(Debug, Default, cucumber::World)]
pub(crate) struct World {
    pub(crate) model: Model,
    /// What the reporters of the scenario's journeys received.
    pub(crate) recorders: Recorders,
    /// How many times the scenario's steps ran.
    pub(crate) steps_run: StepsRun,
    /// What building the scenario's workflow gave, once it was admitted or listed.
    pub(crate) admission: Option<Admission>,
}

/// What building the scenario's workflow gave.
#[derive(Debug)]
pub(crate) enum Admission {
    Admitted(WorkflowDescriptor<ScriptedWorkflow>),
    Refused(Violations),
}
