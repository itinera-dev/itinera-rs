//! The columns that say why: a reason, the abort of a journey, or its failure.

use itinera::event::{
    FailureRequester, JourneyAbort, JourneyFailure, MissingData, Source, Supplier,
};
use itinera::journey::LastFailure;
use itinera::report::GlobalMetadata;
use itinera::step::Reason;
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

    /// A request without a key, from a step hook or a rescue step, named in the key cell by what
    /// it requested.
    fn unkeyed(&mut self, requested: &str, requester: &FailureRequester) {
        self.set("key", requested);
        match requester {
            FailureRequester::StepHook {
                policy, hook, step, ..
            } => {
                self.set("policy", policy);
                self.set("hook", hook);
                self.set("step", step);
            }
            FailureRequester::RescueStep { step, .. } => self.set("step", step),
            _ => {}
        }
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
            JourneyAbort::StepCouldNotBeBuilt {
                supplier: Some(supplier),
                ..
            } => self.supplier(supplier),
            JourneyAbort::PolicyCouldNotBeBuilt { policy, .. } => self.set("policy", policy),
            JourneyAbort::RequiredDataMissing { missing, .. } => match missing {
                MissingData::Key { key, requester, .. } => {
                    self.set("key", key);
                    self.requester(requester);
                }
                MissingData::Reason { requester, .. } => {
                    self.unkeyed("failure reason", requester);
                }
                MissingData::Error { requester, .. } => self.unkeyed("error", requester),
                _ => {}
            },
            JourneyAbort::WrongType { key, requester, .. } => {
                self.set("key", key);
                self.requester(requester);
            }
            JourneyAbort::ReservedMetadataKey { source, .. } => {
                self.set("key", GlobalMetadata::JOURNEY_ID);
                if let Source::Hook(hook) = source {
                    self.hook(hook);
                }
            }
            _ => {}
        }
    }

    /// The input adapter or the service provider that failed while a step was built.
    fn supplier(&mut self, supplier: &Supplier) {
        match supplier {
            Supplier::Adapter { adapter, key, .. } => {
                self.set("adapter", adapter);
                self.set("key", key);
            }
            Supplier::Provider { provider, .. } => self.set("provider", provider),
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
