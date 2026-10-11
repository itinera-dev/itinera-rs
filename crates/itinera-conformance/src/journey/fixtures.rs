//! Fixtures the tests of several modules share: a journey run from a script, and the attempts
//! a script states.

use itinera::event::Event;

use super::run;
use crate::executor::Executor;
use crate::model::{Attempt, AttemptOutcome, Model, Reason};
use crate::world::World;

/// The events the sentences about the whole stream read, from one journey of a workflow with
/// these steps, scripted by `script` and run under the executor, on a runtime like the
/// runner's own, whose steps may block on their script.
pub(crate) fn ran(executor: Executor, steps: &[&str], script: fn(&mut Model)) -> Vec<Event> {
    let mut world = World {
        executor: Some(executor),
        ..World::default()
    };
    let steps = steps.iter().copied().map(str::to_owned).collect();
    world.model.declare("orders".to_owned(), steps).unwrap();
    script(&mut world.model);
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(run(&mut world, 1))
        .unwrap();
    world.recorders.stream(&world.model).unwrap().events()
}

pub(crate) fn attempt(outcome: AttemptOutcome) -> Attempt {
    Attempt {
        outcome,
        contributes: Vec::new(),
    }
}

pub(crate) fn timed_out() -> AttemptOutcome {
    AttemptOutcome::Failure {
        reason: Reason {
            code: "timeout".to_owned(),
            message: None,
            details: None,
        },
        retriable: true,
    }
}
