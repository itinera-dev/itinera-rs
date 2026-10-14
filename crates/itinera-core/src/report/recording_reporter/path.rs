//! A journey's path: the line each event of its flow reads as.

use crate::event::{Event, EventBody, GiveUpCause, JourneyAbort, JourneyFailure};
use crate::policy::PolicyName;
use crate::step::{Reason, StepAttempt, StepName};

/// The line an event reads as in a path, or `None` for an event outside the flow.
pub(super) fn line(event: &Event) -> Option<String> {
    match &event.body {
        EventBody::StepSucceeded { step } => Some(format!("{} succeeded", attempt(step))),
        EventBody::StepFailed { step, reason } => {
            Some(format!("{} failed: {}", attempt(step), reason.code()))
        }
        EventBody::StepSkipped { step, reason } => Some(skipped(step, reason.as_ref())),
        EventBody::StepAbnormalTermination { step, message } => Some(format!(
            "{} terminated abnormally: {message}",
            attempt(step)
        )),
        EventBody::StepRetrying { step, .. } => Some(format!("{} retrying", step.step)),
        EventBody::StepGivenUp { step, cause } => {
            Some(format!("{} given up: {}", step.step, given_up(cause)))
        }
        EventBody::StepRescuing { step, rescue, .. } => {
            Some(format!("{} rescuing with {rescue}", step.step))
        }
        EventBody::StepRescued { step, rescue } => {
            Some(format!("{} rescued by {rescue}", step.step))
        }
        EventBody::JourneySucceeded { decided_by } => Some(succeeded(decided_by.as_ref())),
        EventBody::JourneyFailed { step, failure } => Some(failed(*step, failure)),
        EventBody::JourneyAborted { abort } => Some(aborted(abort)),
        EventBody::JourneyStarted { .. }
        | EventBody::AttemptStarted { .. }
        | EventBody::InputAdapterSupplied { .. }
        | EventBody::InputAdapterFailed { .. }
        | EventBody::OptionalInputAbsent { .. }
        | EventBody::ServiceProvided { .. }
        | EventBody::ServiceProviderFailed { .. }
        | EventBody::HookCalled { .. }
        | EventBody::ContributionCommitted { .. }
        | EventBody::ContributionsDiscarded { .. }
        | EventBody::DataOverwritten { .. }
        | EventBody::GlobalMetadataSet { .. }
        | EventBody::StepInfo { .. }
        | EventBody::StepWarning { .. }
        | EventBody::StepError { .. }
        | EventBody::JourneyInfo { .. }
        | EventBody::JourneyWarning { .. }
        | EventBody::JourneyError { .. } => None,
    }
}

fn attempt(step: &StepAttempt) -> String {
    format!("{} attempt {}", step.step, step.attempt)
}

fn skipped(step: &StepAttempt, reason: Option<&Reason>) -> String {
    match reason {
        Some(reason) => format!("{} skipped: {}", attempt(step), reason.code()),
        None => format!("{} skipped", attempt(step)),
    }
}

fn given_up(cause: &GiveUpCause) -> String {
    match cause {
        GiveUpCause::FailWorkflow { decided_by, .. } => {
            format!("{cause} from {}", decided_by.policy)
        }
        GiveUpCause::Failure | GiveUpCause::AbnormalTermination | GiveUpCause::RetriesExhausted => {
            cause.to_string()
        }
    }
}

fn succeeded(decided_by: Option<&PolicyName>) -> String {
    match decided_by {
        Some(policy) => format!("journey succeeded, decided by {policy}"),
        None => "journey succeeded".to_string(),
    }
}

fn failed(step: StepName, failure: &JourneyFailure) -> String {
    match failure {
        JourneyFailure::Failure(reason) | JourneyFailure::FailWorkflow { reason, .. } => {
            format!("journey failed at {step}: {}", reason.code())
        }
        JourneyFailure::RetriesExhausted(_) | JourneyFailure::AbnormalTermination(_) => {
            format!("journey failed at {step}: {}", failure.cause())
        }
    }
}

fn aborted(abort: &JourneyAbort) -> String {
    match abort.step() {
        Some(step) => format!("journey aborted at {step}: {}", abort.reason()),
        None => format!("journey aborted: {}", abort.reason()),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::event::fixtures::event;
    use crate::event::{DecidingHook, GiveUpHook, RescueCause};
    use crate::journey::LastFailure;
    use crate::policy::{RetryCause, StepHook};

    fn charge() -> StepAttempt {
        StepAttempt::first(StepName::new("charge"))
    }

    fn declined() -> Reason {
        Reason::new("declined")
    }

    #[rstest]
    #[case::a_success(EventBody::StepSucceeded { step: charge() }, "charge attempt 1 succeeded")]
    #[case::a_failure(
        EventBody::StepFailed { step: charge().next(), reason: declined() },
        "charge attempt 2 failed: declined"
    )]
    #[case::a_skip_without_a_reason(
        EventBody::StepSkipped { step: charge(), reason: None },
        "charge attempt 1 skipped"
    )]
    #[case::a_skip_with_a_reason(
        EventBody::StepSkipped { step: charge(), reason: Some(Reason::new("paid")) },
        "charge attempt 1 skipped: paid"
    )]
    #[case::an_abnormal_termination(
        EventBody::StepAbnormalTermination { step: charge(), message: "timeout".to_string() },
        "charge attempt 1 terminated abnormally: timeout"
    )]
    #[case::a_retry(
        EventBody::StepRetrying { step: charge(), cause: RetryCause::Failure },
        "charge retrying"
    )]
    #[case::a_step_given_up_by_its_budget(
        EventBody::StepGivenUp { step: charge(), cause: GiveUpCause::RetriesExhausted },
        "charge given up: retries exhausted"
    )]
    #[case::a_step_given_up_by_a_hook(
        EventBody::StepGivenUp {
            step: charge(),
            cause: GiveUpCause::FailWorkflow {
                decided_by: DecidingHook {
                    policy: PolicyName::from("fraud_check"),
                    hook: GiveUpHook::OnStepRetry,
                },
                reason: Reason::new("fraud"),
            },
        },
        "charge given up: FailWorkflow from fraud_check"
    )]
    #[case::a_rescue_starting(
        EventBody::StepRescuing {
            step: charge(),
            cause: RescueCause::Failure,
            rescue: StepName::new("charge_rescue"),
        },
        "charge rescuing with charge_rescue"
    )]
    #[case::a_rescue_ending(
        EventBody::StepRescued { step: charge(), rescue: StepName::new("charge_rescue") },
        "charge rescued by charge_rescue"
    )]
    #[case::a_success_of_the_journey(
        EventBody::JourneySucceeded { decided_by: None },
        "journey succeeded"
    )]
    #[case::a_success_a_hook_decided(
        EventBody::JourneySucceeded { decided_by: Some(PolicyName::from("early_exit")) },
        "journey succeeded, decided by early_exit"
    )]
    #[case::a_failure_of_the_journey(
        EventBody::JourneyFailed {
            step: StepName::new("charge"),
            failure: JourneyFailure::Failure(declined()),
        },
        "journey failed at charge: declined"
    )]
    #[case::a_failure_once_retries_are_exhausted(
        EventBody::JourneyFailed {
            step: StepName::new("charge"),
            failure: JourneyFailure::RetriesExhausted(LastFailure::Reason(declined())),
        },
        "journey failed at charge: retries exhausted"
    )]
    #[case::a_failure_by_an_abnormal_termination(
        EventBody::JourneyFailed {
            step: StepName::new("charge"),
            failure: JourneyFailure::AbnormalTermination("timeout".to_string()),
        },
        "journey failed at charge: abnormal termination"
    )]
    #[case::a_failure_a_hook_decided(
        EventBody::JourneyFailed {
            step: StepName::new("charge"),
            failure: JourneyFailure::FailWorkflow {
                decided_by: DecidingHook {
                    policy: PolicyName::from("fraud_check"),
                    hook: StepHook::OnStepSuccess,
                },
                reason: Reason::new("fraud"),
            },
        },
        "journey failed at charge: fraud"
    )]
    #[case::an_abort_concerning_a_step(
        EventBody::JourneyAborted {
            abort: JourneyAbort::StepCouldNotBeBuilt {
                step: StepName::new("charge"),
                provider: None,
                error: "no gateway".to_string(),
            },
        },
        "journey aborted at charge: step could not be built"
    )]
    #[case::an_abort_concerning_no_step(
        EventBody::JourneyAborted {
            abort: JourneyAbort::ReporterFailed { step: None, error: "disk full".to_string() },
        },
        "journey aborted: reporter failed"
    )]
    fn each_event_of_the_flow_reads_as_its_line(#[case] body: EventBody, #[case] expected: &str) {
        assert_eq!(line(&event(1, body)).as_deref(), Some(expected));
    }

    #[rstest]
    #[case::a_journey_starting(EventBody::JourneyStarted { initial_keys: Vec::new() })]
    #[case::an_attempt_starting(EventBody::AttemptStarted { step: charge() })]
    #[case::a_discard(EventBody::ContributionsDiscarded { step: charge() })]
    #[case::an_event_a_step_emits(EventBody::StepInfo {
        step: charge(),
        message: "charging".to_string(),
        data: None,
    })]
    fn an_event_outside_the_flow_has_no_line(#[case] body: EventBody) {
        assert_eq!(line(&event(1, body)), None);
    }
}
