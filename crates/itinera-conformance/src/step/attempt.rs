//! One attempt of a scripted step, which does what the script says, then ends as its ending
//! says.

use futures::executor::block_on;
use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::step::{AsyncStep, AsyncStepReporter, Outcome, Step, StepReporter};

use super::ending::{End, Ending};
use super::reporter::Emits;
use super::script::{Action, Script};
use super::{StepsRun, contribute};

/// One attempt of a scripted step, with the handles its attempt gave it.
#[derive(derive_more::Debug)]
pub(crate) struct Attempting<'a, R> {
    pub(super) script: &'a Script,
    pub(super) ending: &'a Ending,
    pub(super) contributor: Contributor<'a>,
    #[debug(skip)]
    pub(super) reporter: R,
    pub(super) runs: &'a StepsRun,
}

impl<'a, R: Emits> Attempting<'a, R> {
    /// Does what the script says, then contributes what the attempt's ending says, and returns
    /// how it ends.
    pub(super) async fn perform(self) -> Result<&'a End, Error> {
        let Self {
            script,
            ending,
            mut contributor,
            mut reporter,
            runs,
        } = self;
        runs.one_more();
        for action in &script.actions {
            match action {
                Action::Contribute(key, value) => contribute(&mut contributor, key, value),
                Action::Emit {
                    level,
                    message,
                    data,
                } => {
                    let emitted = reporter.emit(*level, message.clone(), data.clone()).await;
                    if !script.ignores_failed_emits {
                        emitted?;
                    }
                }
            }
        }
        for (key, value) in &ending.contributes {
            contribute(&mut contributor, key, value);
        }
        Ok(&ending.end)
    }
}

impl Step for Attempting<'_, StepReporter<'_>> {
    fn run(self) -> Result<Outcome, Error> {
        block_on(self.perform()).and_then(End::outcome)
    }
}

impl AsyncStep for Attempting<'_, AsyncStepReporter<'_>> {
    async fn run(self) -> Result<Outcome, Error> {
        self.perform().await.and_then(End::outcome)
    }
}
