//! What cucumber keeps for one scenario while its sentences run.

use crate::declaration::Admission;
use crate::executor::Executor;
use crate::fixture::Fixture;
use crate::journey::Journey;
use crate::model::{Model, ModelError};
use crate::record::Recorders;
use crate::step::StepsRun;
use crate::testing::Tested;
use crate::witness::Witness;

/// The state of one scenario, made new for each.
#[derive(Debug, Default, cucumber::World)]
pub(crate) struct World {
    pub(crate) model: Model,
    /// The workflow the macros declare that the scenario names, if it names one.
    pub(crate) fixture: Option<Fixture>,
    /// The executor this copy of the scenario runs under, from its tag.
    pub(crate) executor: Option<Executor>,
    /// What the reporters of the scenario's journeys received.
    pub(crate) recorders: Recorders,
    /// How many times the scenario's steps ran.
    pub(crate) steps_run: StepsRun,
    /// What the scenario's steps, hooks and roles received and did.
    pub(crate) witness: Witness,
    /// What building the scenario's workflow gave, once it was admitted or listed.
    pub(crate) admission: Option<Admission>,
    /// The journeys the scenario ran, in order.
    pub(crate) journeys: Vec<Journey>,
    /// What the scenario's use of test support gave.
    pub(crate) tested: Tested,
}

impl World {
    /// What the scenario's scripted steps, hooks and roles received and did. A workflow the
    /// macros declare has no script to record it.
    pub(crate) fn witnessed(&self) -> Result<&Witness, ModelError> {
        match self.fixture {
            Some(fixture) => Err(ModelError::Unscripted(fixture)),
            None => Ok(&self.witness),
        }
    }

    /// How many times the scenario's scripted steps ran. A workflow the macros declare has no
    /// script to count them.
    pub(crate) fn steps_run(&self) -> Result<&StepsRun, ModelError> {
        match self.fixture {
            Some(fixture) => Err(ModelError::Unscripted(fixture)),
            None => Ok(&self.steps_run),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_named_fixture_has_no_script_to_read() {
        let world = World {
            fixture: Some(Fixture::Payment),
            ..World::default()
        };
        let unscripted = Some(ModelError::Unscripted(Fixture::Payment));
        assert_eq!(world.witnessed().err(), unscripted);
        assert_eq!(world.steps_run().err(), unscripted);
    }
}
