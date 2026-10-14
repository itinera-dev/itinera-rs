//! A scripted step run alone, outside any journey, as a test runs one: built by the test, with
//! the handles that the recorders of itinera's test support lend.

use std::num::NonZeroU32;

use itinera::error::Error;
use itinera::journey::{Contributions, Contributor};
use itinera::step::{AsyncStep, Step, StepEventReporter};

use super::Scripted;
use super::attempt::Attempting;
use crate::executor::Executor;

impl Scripted {
    /// Runs attempt 1 of the step with a contributor the contributions recorder lends and a
    /// reporter the step event reporter lends, in the mode of the executor. It carries on after
    /// any call to its reporter that is interrupted. A step that requests services is never run
    /// alone, since a test would have to give it services of its own.
    pub(crate) async fn run_alone(
        &self,
        contributions: &mut Contributions,
        events: &mut StepEventReporter,
        executor: Executor,
    ) -> Result<(), Error> {
        if !self.script.services.is_empty() {
            return Err(Error::msg(format!(
                "the step {} requests services, which a step run alone is not given",
                self.name
            )));
        }
        let contributor = contributions.contributor();
        match executor {
            Executor::Local => Step::run(self.alone(contributor, events.reporter())).map(drop),
            Executor::AsyncLocal => {
                AsyncStep::run(self.alone(contributor, events.async_reporter()))
                    .await
                    .map(drop)
            }
        }
    }

    /// The step's first attempt, with these handles, carrying on after an interrupted call.
    fn alone<'a, R>(&'a self, contributor: Contributor<'a>, reporter: R) -> Attempting<'a, R> {
        Attempting {
            step: &self.name,
            attempt: NonZeroU32::MIN,
            services: Vec::new(),
            script: &self.script,
            end: self.script.end(0),
            contributor: Some(contributor),
            reporter: Some(reporter),
            carries_on: true,
            runs: &self.runs,
            witness: &self.witness,
        }
    }
}
