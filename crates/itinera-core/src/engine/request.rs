//! Turning what was found for a request into its value, or into the abort it causes, naming who
//! requested it: a step, an input adapter or a hook.

use super::end::End;
use super::{Delivery, Journey};
use crate::event::{self, EventBody, HookSource, JourneyAbort, RequestSource};
use crate::journey::{Abort, MissingData, Requester};
use crate::step::{InputNeed, Requirement, StepAttempt};
use crate::value::AnyValue;
use crate::workflow::AdapterName;

/// Who requests a value, as the result, the abort and the events name it.
pub(super) struct Requesting {
    requester: Requester,
    reported: event::Requester,
    source: RequestSource,
}

impl Requesting {
    /// A step, for one of its inputs during this attempt.
    pub(super) fn step(attempt: &StepAttempt) -> Self {
        Self {
            requester: Requester::Step,
            reported: event::Requester::Step { step: attempt.step },
            source: RequestSource::Step(attempt.clone()),
        }
    }

    /// An input adapter, for an input of the step it is building in this attempt.
    pub(super) fn adapter(adapter: AdapterName, attempt: &StepAttempt) -> Self {
        Self {
            requester: Requester::Adapter { adapter },
            reported: event::Requester::Adapter {
                adapter,
                step: attempt.step,
            },
            source: RequestSource::Adapter {
                adapter,
                step: attempt.clone(),
            },
        }
    }

    /// A hook of a policy.
    pub(super) fn hook(hook: &HookSource) -> Self {
        let (requester, reported) = match *hook {
            HookSource::Step {
                policy,
                hook,
                ref step,
            } => (
                Requester::StepHook { policy, hook },
                event::Requester::StepHook {
                    policy,
                    hook,
                    step: step.step,
                },
            ),
            HookSource::Workflow { policy, hook } => (
                Requester::WorkflowHook { policy, hook },
                event::Requester::WorkflowHook { policy, hook },
            ),
        };
        Self {
            requester,
            reported,
            source: RequestSource::Hook(hook.clone()),
        }
    }
}

impl<D: Delivery> Journey<D> {
    /// Turns what was found for one request into its value, or the abort it causes.
    pub(super) async fn request(
        &mut self,
        requesting: &Requesting,
        need: &InputNeed,
        found: Option<AnyValue>,
    ) -> Result<Option<AnyValue>, End> {
        let key = need.key();
        match found {
            Some(value) if need.accepts(&value) => Ok(Some(value)),
            Some(_) => Err(wrong_type(key, requesting)),
            None => match need.requirement() {
                Requirement::Required => Err(missing(key, requesting)),
                Requirement::Optional => self.absent(key, requesting.source.clone()).await,
            },
        }
    }

    /// Reports that an optional request has no value, which its requester receives absent.
    async fn absent(
        &mut self,
        key: &str,
        requester: RequestSource,
    ) -> Result<Option<AnyValue>, End> {
        self.emit(EventBody::OptionalInputAbsent {
            key: key.to_owned(),
            requester,
        })
        .await?;
        Ok(None)
    }
}

/// The abort for a required request without a value, naming who requested it.
fn missing(key: &str, requesting: &Requesting) -> End {
    End::aborted(
        Abort::RequiredDataMissing(MissingData::Key {
            key: key.to_owned(),
            requester: requesting.requester.clone(),
        }),
        JourneyAbort::RequiredDataMissing {
            missing: event::MissingData::Key {
                key: key.to_owned(),
                requester: requesting.reported.clone(),
            },
        },
    )
}

/// The abort for a value of the wrong type, naming who requested it: the input adapter that
/// supplied it, or the step, input adapter or hook that read it.
pub(super) fn wrong_type(key: &str, requesting: &Requesting) -> End {
    End::aborted(
        Abort::WrongType {
            key: key.to_owned(),
            requester: requesting.requester.clone(),
        },
        JourneyAbort::WrongType {
            key: key.to_owned(),
            requester: requesting.reported.clone(),
        },
    )
}
