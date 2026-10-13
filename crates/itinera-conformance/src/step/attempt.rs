//! One attempt of a scripted step, which does what the script says, then ends as its ending
//! says.

use std::num::NonZeroU32;

use futures::executor::block_on;
use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::step::{AsyncStep, AsyncStepReporter, Ending, Outcome, Step, StepReporter};

use super::attempt_end::AttemptEnd;
use super::reporter::Emits;
use super::script::{Action, Script};
use super::service::Lent;
use super::{StepsRun, contribute};
use crate::witness::{Given, Witness};

/// One attempt of a scripted step, with the handles its attempt gave it.
#[derive(derive_more::Debug)]
pub(crate) struct Attempting<'a, R> {
    pub(super) step: &'a str,
    pub(super) attempt: NonZeroU32,
    /// The services it was lent, in the order it requests them.
    pub(super) services: Vec<Lent<'a>>,
    pub(super) script: &'a Script,
    pub(super) end: &'a AttemptEnd,
    pub(super) contributor: Contributor<'a>,
    #[debug(skip)]
    pub(super) reporter: R,
    pub(super) runs: &'a StepsRun,
    pub(super) witness: &'a Witness,
}

impl<'a, R: Emits> Attempting<'a, R> {
    /// Uses each service it was lent, does what the script says, then makes the contributions
    /// its end states, and returns its ending.
    pub(super) async fn perform(self) -> Result<&'a Ending, Error> {
        let Self {
            step,
            attempt,
            services,
            script,
            end,
            mut contributor,
            mut reporter,
            runs,
            witness,
        } = self;
        runs.one_more();
        for service in &services {
            witness.given(Given {
                step: step.to_owned(),
                attempt,
                key: service.key().to_owned(),
                label: service.label()?,
            });
        }
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
                Action::SetGlobalMetadata(key, value) => {
                    let set = reporter
                        .set_global_metadata(key.clone(), value.clone())
                        .await;
                    if !script.ignores_failed_emits {
                        set?;
                    }
                }
            }
        }
        for (key, value) in &end.contributes {
            contribute(&mut contributor, key, value);
        }
        Ok(&end.ending)
    }
}

impl Step for Attempting<'_, StepReporter<'_>> {
    fn run(self) -> Result<Outcome, Error> {
        block_on(self.perform()).and_then(Ending::outcome)
    }
}

impl AsyncStep for Attempting<'_, AsyncStepReporter<'_>> {
    async fn run(self) -> Result<Outcome, Error> {
        self.perform().await.and_then(Ending::outcome)
    }
}
