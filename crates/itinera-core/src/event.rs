//! Events: the record of what happens in a journey, and what only events carry.

use std::num::NonZeroU64;

use crate::journey::JourneyId;
use crate::workflow::WorkflowName;

mod abort;
mod body;
mod decision;
mod failure;
mod requester;
mod source;
mod timestamp;

pub use abort::{JourneyAbort, MissingData};
pub use body::EventBody;
pub use decision::{DecidingHook, GiveUpCause, GiveUpHook};
pub use failure::JourneyFailure;
pub use requester::Requester;
pub use source::{HookSource, RequestSource, Source};
pub use timestamp::Timestamp;

/// One event of a journey's event stream.
///
/// Events are made only by itinera's executors. Every event carries a sequence number,
/// increasing from 1 within the journey, a timestamp, the journey ID and the workflow name;
/// what else it carries depends on its [`body`](Event::body).
///
/// Events carry no format of their own: each reporter writes them as it chooses.
///
/// # Examples
///
/// ```
/// use itinera::event::{Event, EventBody};
///
/// fn summary(event: &Event) -> String {
///     match &event.body {
///         EventBody::StepFailed { step, reason, .. } => {
///             format!("{} failed: {}", step.step, reason.code())
///         }
///         _ => format!("#{} {}", event.sequence, event.kind()),
///     }
/// }
/// ```
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Event {
    /// The event's position in the journey's event stream, from 1.
    pub sequence: NonZeroU64,
    /// When the event was emitted.
    pub timestamp: Timestamp,
    /// The journey's ID.
    pub journey_id: JourneyId,
    /// The workflow's name.
    pub workflow: WorkflowName,
    /// What the event says.
    pub body: EventBody,
}
impl Event {
    /// The event's kind, in snake_case, for example `step_failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::event::Event;
    ///
    /// fn is_last(event: &Event) -> bool {
    ///     matches!(event.kind(), "journey_succeeded" | "journey_failed" | "journey_aborted")
    /// }
    /// ```
    pub fn kind(&self) -> &'static str {
        self.body.kind()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::num::NonZeroU32;
    use std::time::{Duration, UNIX_EPOCH};

    use super::*;
    use crate::journey::LastFailure;
    use crate::policy::{PolicyName, RetryCause, StepHook, WorkflowHook};
    use crate::step::{Reason, StepAttempt, StepName};
    use crate::workflow::AdapterName;

    pub(crate) fn event(sequence: u64, body: EventBody) -> Event {
        Event {
            sequence: NonZeroU64::new(sequence).unwrap(),
            timestamp: Timestamp::from(UNIX_EPOCH + Duration::from_millis(1_700_000_000_123)),
            journey_id: JourneyId::from("order-42"),
            workflow: WorkflowName::from("orders"),
            body,
        }
    }

    fn charge(attempt: u32) -> StepAttempt {
        StepAttempt {
            step: StepName::new("charge"),
            attempt: NonZeroU32::new(attempt).unwrap(),
        }
    }

    fn audit_step(hook: StepHook, step: StepAttempt) -> HookSource {
        HookSource::Step {
            policy: PolicyName::from("audit"),
            hook,
            step,
        }
    }

    fn audit_workflow(hook: WorkflowHook) -> HookSource {
        HookSource::Workflow {
            policy: PolicyName::from("audit"),
            hook,
        }
    }

    fn every_kind() -> Vec<EventBody> {
        let reason = || Reason::new("declined");
        vec![
            EventBody::JourneyStarted {
                initial_keys: vec!["amount".to_string()],
            },
            EventBody::AttemptStarted { step: charge(1) },
            EventBody::InputAdapterSupplied {
                step: charge(1),
                key: "amount".to_string(),
                adapter: AdapterName::from("pricing"),
            },
            EventBody::InputAdapterFailed {
                step: charge(1),
                key: "amount".to_string(),
                adapter: AdapterName::from("pricing"),
            },
            EventBody::OptionalInputAbsent {
                key: "discount".to_string(),
                requester: RequestSource::Step(charge(1)),
            },
            EventBody::StepSucceeded { step: charge(1) },
            EventBody::StepFailed {
                step: charge(1),
                retriable: true,
                reason: reason(),
            },
            EventBody::StepSkipped {
                step: charge(1),
                reason: None,
            },
            EventBody::StepAbnormalTermination {
                step: charge(1),
                message: "boom".to_string(),
            },
            EventBody::HookCalled {
                hook: audit_step(StepHook::OnStepSuccess, charge(1)),
                lifecycle: None,
            },
            EventBody::ContributionCommitted {
                key: "receipt".to_string(),
                source: Source::Step(charge(1)),
            },
            EventBody::ContributionsDiscarded { step: charge(1) },
            EventBody::DataOverwritten {
                key: "receipt".to_string(),
                source: Source::Step(charge(1)),
            },
            EventBody::JourneyAborted {
                abort: JourneyAbort::ReporterFailed {
                    step: None,
                    error: "disk full".to_string(),
                },
            },
            EventBody::StepRetrying {
                step: charge(1),
                cause: RetryCause::RetriableFailure,
            },
            EventBody::StepGivenUp {
                step: charge(2),
                cause: GiveUpCause::RetriesExhausted,
            },
            EventBody::JourneySucceeded { decided_by: None },
            EventBody::JourneyFailed {
                step: StepName::new("charge"),
                failure: JourneyFailure::RetriesExhausted(LastFailure::Reason(reason())),
            },
            EventBody::StepInfo {
                step: charge(1),
                message: "charging".to_string(),
                data: None,
            },
            EventBody::StepWarning {
                step: charge(1),
                message: "slow".to_string(),
                data: None,
            },
            EventBody::StepError {
                step: charge(1),
                message: "odd".to_string(),
                data: None,
            },
            EventBody::JourneyInfo {
                hook: audit_workflow(WorkflowHook::OnWorkflowSuccess),
                message: "done".to_string(),
                data: None,
            },
            EventBody::JourneyWarning {
                hook: audit_workflow(WorkflowHook::OnWorkflowSuccess),
                message: "late".to_string(),
                data: None,
            },
            EventBody::JourneyError {
                hook: audit_workflow(WorkflowHook::OnWorkflowFailure),
                message: "lost".to_string(),
                data: None,
            },
        ]
    }

    #[test]
    fn every_kind_of_event_has_its_own_kind() {
        let kinds: std::collections::HashSet<_> =
            every_kind().iter().map(EventBody::kind).collect();
        assert_eq!(kinds.len(), 24);
        let event = event(1, EventBody::AttemptStarted { step: charge(1) });
        assert_eq!(event.kind(), "attempt_started");
    }
}
