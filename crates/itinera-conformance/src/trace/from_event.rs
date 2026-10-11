//! The line of an event: the columns each kind of event fills.

use itinera::event::{Event, EventBody, GiveUpCause, RequestSource};
use serde_json::Value;

use super::Line;

impl From<&Event> for Line {
    fn from(event: &Event) -> Self {
        let mut line = Self {
            event: event.kind().to_owned(),
            ..Self::default()
        };
        match &event.body {
            EventBody::AttemptStarted { step, .. }
            | EventBody::StepSucceeded { step, .. }
            | EventBody::ContributionsDiscarded { step, .. } => line.attempt(step),
            EventBody::StepAbnormalTermination { step, message, .. } => {
                line.attempt(step);
                line.carried.push(Value::from(message.as_str()));
            }
            EventBody::InputAdapterSupplied {
                step, key, adapter, ..
            }
            | EventBody::InputAdapterFailed {
                step, key, adapter, ..
            } => {
                line.attempt(step);
                line.set("key", key);
                line.set("adapter", adapter);
            }
            EventBody::OptionalInputAbsent { key, requester, .. } => {
                line.set("key", key);
                match requester {
                    RequestSource::Step(step) => line.attempt(step),
                    RequestSource::Hook(hook) => line.hook(hook),
                    RequestSource::Adapter { adapter, step, .. } => {
                        line.attempt(step);
                        line.set("adapter", adapter);
                    }
                    _ => {}
                }
            }
            EventBody::StepFailed {
                step,
                retriable,
                reason,
                ..
            } => {
                line.attempt(step);
                line.set("retriable", retriable);
                line.reason(reason);
            }
            EventBody::StepSkipped { step, reason, .. } => {
                line.attempt(step);
                if let Some(reason) = reason {
                    line.reason(reason);
                }
            }
            EventBody::HookCalled {
                hook, lifecycle, ..
            } => {
                line.hook(hook);
                match lifecycle {
                    Some(lifecycle) => line.set("lifecycle", lifecycle),
                    None => line.set("lifecycle", "none"),
                }
            }
            EventBody::ContributionCommitted { key, source, .. }
            | EventBody::DataOverwritten { key, source, .. } => {
                line.set("key", key);
                line.source(source);
            }
            EventBody::JourneyAborted { abort, .. } => line.abort(abort),
            EventBody::StepRetrying { step, cause, .. } => {
                line.attempt(step);
                line.set("cause", cause);
                line.set("decided by", "default");
            }
            EventBody::StepGivenUp { step, cause, .. } => {
                line.attempt(step);
                line.set("cause", cause);
                match cause {
                    GiveUpCause::FailWorkflow {
                        decided_by, reason, ..
                    } => {
                        line.set(
                            "decided by",
                            format!("{}, {}", decided_by.policy, decided_by.hook),
                        );
                        line.reason(reason);
                    }
                    _ => line.set("decided by", "default"),
                }
            }
            EventBody::JourneySucceeded { decided_by, .. } => match decided_by {
                Some(policy) => line.set("decided by", format!("{policy}, on step success")),
                None => line.set("decided by", "default"),
            },
            EventBody::JourneyFailed { step, failure, .. } => {
                line.set("step", step);
                line.failure(failure);
            }
            EventBody::StepInfo {
                step,
                message,
                data,
                ..
            }
            | EventBody::StepWarning {
                step,
                message,
                data,
                ..
            }
            | EventBody::StepError {
                step,
                message,
                data,
                ..
            } => {
                line.attempt(step);
                line.emitted(message, data.as_ref());
            }
            EventBody::JourneyInfo {
                hook,
                message,
                data,
                ..
            }
            | EventBody::JourneyWarning {
                hook,
                message,
                data,
                ..
            }
            | EventBody::JourneyError {
                hook,
                message,
                data,
                ..
            } => {
                line.hook(hook);
                line.emitted(message, data.as_ref());
            }
            _ => {}
        }
        line
    }
}

impl Line {
    fn emitted(&mut self, message: &str, data: Option<&itinera::value::AnyValue>) {
        self.set("message", message);
        self.data = data.and_then(as_json);
    }
}

/// Event data as JSON, to compare with what a table writes.
fn as_json(data: &itinera::value::AnyValue) -> Option<Value> {
    serde_json::to_value(data).ok()
}
