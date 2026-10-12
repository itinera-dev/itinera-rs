use std::num::NonZeroU32;

use super::*;
use crate::event::fixtures::event;
use crate::journey::LastFailure;
use crate::policy::{StepHook, WorkflowHook};

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
        EventBody::GlobalMetadataSet {
            key: "user_id".to_string(),
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
        EventBody::StepRescuing {
            step: charge(1),
            cause: RescueCause::Failure,
            rescue: StepName::new("charge_rescue"),
        },
        EventBody::StepRescued {
            step: charge(1),
            rescue: StepName::new("charge_rescue"),
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
    let kinds: std::collections::HashSet<_> = every_kind().iter().map(EventBody::kind).collect();
    assert_eq!(kinds.len(), 27);
    let event = event(1, EventBody::AttemptStarted { step: charge(1) });
    assert_eq!(event.kind(), "attempt_started");
}
