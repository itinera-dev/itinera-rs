//! One attempt of a scripted step, which does what the script says, then ends as its ending
//! says.

use std::num::NonZeroU32;

use futures::executor::block_on;
use itinera::error::{Error, Interrupted};
use itinera::journey::Contributor;
use itinera::step::{AsyncStep, AsyncStepReporter, Ending, Outcome, Step, StepReporter};

use super::attempt_end::AttemptEnd;
use super::reporter::Emits;
use super::script::{Action, Script};
use super::service::Lent;
use super::{StepsRun, contribute};
use crate::witness::{Caller, Given, Reported, Witness};

/// One attempt of a scripted step, with the handles its attempt gave it.
#[derive(derive_more::Debug)]
pub(crate) struct Attempting<'a, R> {
    pub(super) step: &'a str,
    pub(super) attempt: NonZeroU32,
    /// The services it was lent, in the order it requests them.
    pub(super) services: Vec<Lent<'a>>,
    pub(super) script: &'a Script,
    pub(super) end: &'a AttemptEnd,
    /// Its contributor, unless it declares none.
    pub(super) contributor: Option<Contributor<'a>>,
    /// Its reporter, unless it declares none.
    #[debug(skip)]
    pub(super) reporter: Option<R>,
    /// Whether it carries on with its script after a call to its reporter is interrupted.
    pub(super) carries_on: bool,
    pub(super) runs: &'a StepsRun,
    pub(super) witness: &'a Witness,
}

impl<'a, R: Emits> Attempting<'a, R> {
    /// Uses each service it was lent, does what the script says, then makes the contributions
    /// its end states, and returns its ending. Each call to its reporter is recorded with whether
    /// it was interrupted.
    pub(super) async fn perform(self) -> Result<&'a Ending, Error> {
        let Self {
            step,
            attempt,
            services,
            script,
            end,
            mut contributor,
            mut reporter,
            carries_on,
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
        let reporting = Reporting {
            step,
            witness,
            carries_on,
        };
        for action in &script.actions {
            match action {
                Action::Contribute(key, value) => {
                    contribute(declared(&mut contributor)?, key, value)
                }
                Action::Emit {
                    level,
                    message,
                    data,
                } => {
                    let emitted = declared(&mut reporter)?
                        .emit(*level, message.clone(), data.clone())
                        .await;
                    reporting.called(None, emitted)?;
                }
                Action::SetGlobalMetadata(key, value) => {
                    let set = declared(&mut reporter)?
                        .set_global_metadata(key.clone(), value.clone())
                        .await;
                    reporting.called(Some(key.as_str()), set)?;
                }
            }
        }
        for (key, value) in &end.contributes {
            contribute(declared(&mut contributor)?, key, value);
        }
        Ok(&end.ending)
    }
}

/// How an attempt records the calls to its reporter, and whether it carries on after one is
/// interrupted.
struct Reporting<'a> {
    step: &'a str,
    witness: &'a Witness,
    carries_on: bool,
}

impl Reporting<'_> {
    /// Records the call, setting the key of global metadata if it names one, then gives what it
    /// returned, unless the step carries on.
    fn called(
        &self,
        setting: Option<&str>,
        returned: Result<(), Interrupted>,
    ) -> Result<(), Interrupted> {
        self.witness.reported(Reported {
            caller: Caller::Step(self.step.to_owned()),
            setting: setting.map(str::to_owned),
            interrupted: returned.is_err(),
        });
        if self.carries_on { Ok(()) } else { returned }
    }
}

/// The handle the step declared, or the error of using one it did not declare.
fn declared<T>(handle: &mut Option<T>) -> Result<&mut T, Error> {
    handle
        .as_mut()
        .ok_or_else(|| Error::msg("the scripted step declares no contributor and no reporter"))
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
