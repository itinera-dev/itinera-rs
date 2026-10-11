//! What a step's attempt left for a step hook to request.

use crate::engine::decision::Failed;
use crate::engine::end::End;
use crate::error::Error;
use crate::event::{self, JourneyAbort};
use crate::journey::{Abort, Contributions, FailureRequester, MissingData};
use crate::policy::{PolicyName, StepHook};
use crate::step::{Reason, Requirement, StepName};
use crate::value::AnyValue;

/// What a step's attempt left for a step hook to request, and which hook requests it.
pub(crate) struct Left<'s> {
    pub(super) policy: PolicyName,
    pub(super) hook: StepHook,
    pub(super) step: StepName,
    /// What the attempt contributed, committed or not.
    pub(super) contributed: &'s Contributions,
    /// How the attempt failed, if it did.
    pub(super) failed: Option<&'s Failed>,
}

impl<'s> Left<'s> {
    /// The hook, as the result names who requested what the attempt left.
    fn requester(&self) -> FailureRequester {
        FailureRequester::StepHook {
            policy: self.policy,
            hook: self.hook,
        }
    }

    /// The hook and the step it acts on, as events name who requested what the attempt left.
    fn reported(&self) -> event::FailureRequester {
        event::FailureRequester::StepHook {
            policy: self.policy,
            hook: self.hook,
            step: self.step,
        }
    }

    pub(in crate::engine::hooks) fn contributed(&self, key: &str) -> Option<AnyValue> {
        self.contributed.get(key).cloned()
    }

    /// The attempt's reason, for a hook that requested it as `requirement`.
    pub(in crate::engine::hooks) fn reason(
        &self,
        requirement: Requirement,
    ) -> Result<Option<Reason>, End> {
        match (requirement, self.failed.and_then(Failed::reason)) {
            (Requirement::Required, None) => Err(missing_data(
                MissingData::Reason {
                    requester: self.requester(),
                },
                event::MissingData::Reason {
                    requester: self.reported(),
                },
            )),
            (_, found) => Ok(found.cloned()),
        }
    }

    /// The attempt's error, for a hook that requested it as `requirement`.
    pub(in crate::engine::hooks) fn error(
        &self,
        requirement: Requirement,
    ) -> Result<Option<&'s Error>, End> {
        match (requirement, self.failed.and_then(Failed::error)) {
            (Requirement::Required, None) => Err(missing_data(
                MissingData::Error {
                    requester: self.requester(),
                },
                event::MissingData::Error {
                    requester: self.reported(),
                },
            )),
            (_, found) => Ok(found),
        }
    }
}

/// The abort for the reason or the error a step hook required from an attempt that has none.
fn missing_data(missing: MissingData, reported: event::MissingData) -> End {
    End::aborted(
        Abort::RequiredDataMissing(missing),
        JourneyAbort::RequiredDataMissing { missing: reported },
    )
}
