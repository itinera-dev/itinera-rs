//! The columns that say why: a reason, the abort of a journey, or its failure.

use itinera::event::{JourneyAbort, JourneyFailure, MissingData};
use itinera::journey::LastFailure;
use itinera::policy::{PolicyName, StepHook};
use itinera::step::{Reason, StepName};
use serde_json::Value;

use super::Line;

impl Line {
    pub(super) fn reason(&mut self, reason: &Reason) {
        self.set("code", reason.code());
        if let Some(message) = reason.message() {
            self.carried.push(Value::from(message));
        }
        if let Some(details) = reason.details() {
            self.carried
                .push(serde_json::to_value(details).unwrap_or(Value::Null));
        }
    }

    /// A step hook's request without a key, named in the key cell by what it requested.
    fn unkeyed(&mut self, requested: &str, policy: &PolicyName, hook: &StepHook, step: &StepName) {
        self.set("key", requested);
        self.set("policy", policy);
        self.set("hook", hook);
        self.set("step", step);
    }

    pub(super) fn abort(&mut self, abort: &JourneyAbort) {
        self.set("code", abort.reason());
        if let Some(step) = abort.step() {
            self.set("step", step);
        }
        if let Some(error) = abort.error() {
            self.set("error", error);
        }
        match abort {
            JourneyAbort::PolicyCouldNotBeBuilt { policy, .. } => self.set("policy", policy),
            JourneyAbort::RequiredDataMissing { missing, .. } => match missing {
                MissingData::Key { key, requester, .. } => {
                    self.set("key", key);
                    self.requester(requester);
                }
                MissingData::Reason {
                    policy, hook, step, ..
                } => self.unkeyed("failure reason", policy, hook, step),
                MissingData::Error {
                    policy, hook, step, ..
                } => self.unkeyed("error", policy, hook, step),
                _ => {}
            },
            JourneyAbort::WrongType { key, requester, .. } => {
                self.set("key", key);
                self.requester(requester);
            }
            _ => {}
        }
    }

    pub(super) fn failure(&mut self, failure: &JourneyFailure) {
        self.set("cause", failure.cause());
        self.set("decided by", "default");
        match failure {
            JourneyFailure::Failure(reason) => self.reason(reason),
            JourneyFailure::RetriesExhausted(LastFailure::Reason(reason)) => self.reason(reason),
            JourneyFailure::RetriesExhausted(LastFailure::Error(error))
            | JourneyFailure::AbnormalTermination(error) => self.set("error", error),
            JourneyFailure::FailWorkflow {
                decided_by, reason, ..
            } => {
                self.set(
                    "decided by",
                    format!("{}, {}", decided_by.policy, decided_by.hook),
                );
                self.reason(reason);
            }
            _ => {}
        }
    }
}
