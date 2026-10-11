//! The columns that name where an event comes from: the step and its attempt, the hook, the
//! requester and the source of a contribution.

use itinera::event::{HookSource, Requester, Source};
use itinera::step::StepAttempt;

use super::Line;

impl Line {
    pub(super) fn attempt(&mut self, step: &StepAttempt) {
        self.set("step", step.step);
        self.set("attempt", step.attempt);
    }

    pub(super) fn hook(&mut self, hook: &HookSource) {
        match hook {
            HookSource::Step {
                policy, hook, step, ..
            } => {
                self.attempt(step);
                self.set("policy", policy);
                self.set("hook", hook);
            }
            HookSource::Workflow { policy, hook, .. } => {
                self.set("policy", policy);
                self.set("hook", hook);
            }
            _ => {}
        }
    }

    pub(super) fn requester(&mut self, requester: &Requester) {
        match requester {
            Requester::Step { step, .. } => self.set("step", step),
            Requester::Adapter { adapter, step, .. } => {
                self.set("adapter", adapter);
                self.set("step", step);
            }
            Requester::StepHook {
                policy, hook, step, ..
            } => {
                self.set("policy", policy);
                self.set("hook", hook);
                self.set("step", step);
            }
            Requester::WorkflowHook { policy, hook, .. } => {
                self.set("policy", policy);
                self.set("hook", hook);
            }
            _ => {}
        }
    }

    pub(super) fn source(&mut self, source: &Source) {
        match source {
            Source::Step(step) => {
                self.attempt(step);
                self.set("source", step.step);
            }
            Source::Hook(hook) => {
                self.hook(hook);
                if let (Some(policy), Some(hook)) =
                    (self.cells.get("policy"), self.cells.get("hook"))
                {
                    self.set("source", format!("{policy}, {hook}"));
                }
            }
            _ => {}
        }
    }
}
