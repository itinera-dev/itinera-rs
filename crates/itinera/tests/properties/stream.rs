//! Reading an event stream: how it ends, its attempts, and what each attempt holds.

use std::num::NonZeroUsize;

use itinera::event::{Event, EventBody};
use itinera::journey::StatusKind;
use itinera::step::{StepAttempt, StepName};

/// The status of the journey an event ends, if it ends one.
pub(crate) fn ended_status(event: &Event) -> Option<StatusKind> {
    match event.body {
        EventBody::JourneySucceeded { .. } => Some(StatusKind::Succeeded),
        EventBody::JourneyFailed { .. } => Some(StatusKind::Failed),
        EventBody::JourneyAborted { .. } => Some(StatusKind::Aborted),
        _ => None,
    }
}

pub(crate) fn ends_journey(event: &Event) -> bool {
    ended_status(event).is_some()
}

pub(crate) fn starts_journey(event: &Event) -> bool {
    matches!(event.body, EventBody::JourneyStarted { .. })
}

pub(crate) fn sequence(event: &Event) -> u64 {
    event.sequence.get()
}

/// Whether the event is `journey_succeeded` or `journey_failed`.
pub(crate) fn succeeds_or_fails(event: &Event) -> bool {
    matches!(
        event.body,
        EventBody::JourneySucceeded { .. } | EventBody::JourneyFailed { .. }
    )
}

/// The number of the event at `position`, counting from 1.
pub(crate) fn counted_from_one(position: usize) -> Option<NonZeroUsize> {
    NonZeroUsize::new(position + 1)
}

/// Whether the executor emitted the event: every event except those a step or hook emits.
pub(crate) fn is_engine_event(event: &Event) -> bool {
    !matches!(
        event.body,
        EventBody::StepInfo { .. }
            | EventBody::StepWarning { .. }
            | EventBody::StepError { .. }
            | EventBody::JourneyInfo { .. }
            | EventBody::JourneyWarning { .. }
            | EventBody::JourneyError { .. }
    )
}

/// The event as two executors must both emit it: everything but its journey ID and timestamp.
#[cfg(feature = "async")]
pub(crate) fn untimed(event: &Event) -> String {
    format!("{} {} {:?}", event.sequence, event.workflow, event.body)
}

/// Each attempt's events, from its `attempt_started` up to the next attempt's, or to the end of
/// the stream.
pub(crate) fn attempts(events: &[Event]) -> impl Iterator<Item = &[Event]> {
    events.chunk_by(within_one_attempt).filter(begins_attempt)
}

fn within_one_attempt(_: &Event, next: &Event) -> bool {
    attempt_started(next).is_none()
}

fn begins_attempt(events: &&[Event]) -> bool {
    events.first().and_then(attempt_started).is_some()
}

/// The attempt that the attempt's events start, as a list of one.
pub(crate) fn started(attempt: &[Event]) -> Vec<&StepAttempt> {
    attempt
        .first()
        .and_then(attempt_started)
        .into_iter()
        .collect()
}

/// The attempts that the outcome facts among the events report on.
pub(crate) fn outcome_facts(attempt: &[Event]) -> Vec<&StepAttempt> {
    attempt.iter().filter_map(outcome_fact).collect()
}

/// The attempts that the step decisions among the events decide on.
pub(crate) fn step_decisions(attempt: &[Event]) -> Vec<&StepAttempt> {
    attempt.iter().filter_map(step_decision).collect()
}

/// The step decisions an attempt's events need: one, on that attempt, when it ended in
/// `step_failed` or `step_abnormal_termination`, and none otherwise.
pub(crate) fn needed_decisions(attempt: &[Event]) -> Vec<&StepAttempt> {
    if attempt.iter().any(ends_in_failure) {
        started(attempt)
    } else {
        Vec::new()
    }
}

/// The number of each attempt of the step, in the order they started.
pub(crate) fn attempt_numbers(events: &[Event], step: StepName) -> Vec<u32> {
    events
        .iter()
        .filter_map(attempt_started)
        .filter(|attempt| is_of(attempt, step))
        .map(number)
        .collect()
}

fn attempt_started(event: &Event) -> Option<&StepAttempt> {
    match &event.body {
        EventBody::AttemptStarted { step, .. } => Some(step),
        _ => None,
    }
}

fn outcome_fact(event: &Event) -> Option<&StepAttempt> {
    match &event.body {
        EventBody::StepSucceeded { step, .. }
        | EventBody::StepFailed { step, .. }
        | EventBody::StepSkipped { step, .. }
        | EventBody::StepAbnormalTermination { step, .. } => Some(step),
        _ => None,
    }
}

fn step_decision(event: &Event) -> Option<&StepAttempt> {
    match &event.body {
        EventBody::StepRetrying { step, .. }
        | EventBody::StepRescuing { step, .. }
        | EventBody::StepGivenUp { step, .. } => Some(step),
        _ => None,
    }
}

fn ends_in_failure(event: &Event) -> bool {
    matches!(
        event.body,
        EventBody::StepFailed { .. } | EventBody::StepAbnormalTermination { .. }
    )
}

fn is_of(attempt: &StepAttempt, step: StepName) -> bool {
    attempt.step == step
}

fn number(attempt: &StepAttempt) -> u32 {
    attempt.attempt.get()
}
