//! What step and hook reporters of either mode emit through: the journey, with who emits and the
//! kind of event.

use super::StepAttempt;
use crate::engine::{Emitted, Emitting};
use crate::event::{EventBody, HookSource};
use crate::value::AnyValue;

/// What a step or hook reporter of either mode emits through: the journey, while the attempt or
/// the hook runs.
#[derive(derive_more::Debug)]
pub(crate) struct Reporting<'a> {
    #[debug(skip)]
    emitting: &'a mut dyn Emitting,
    stamp: Stamp,
}

/// Who emits through a reporter, which every event it emits carries: a step's attempt, or a hook.
#[derive(Clone, Debug, derive_more::From)]
pub(crate) enum Stamp {
    Step(StepAttempt),
    Hook(HookSource),
}

/// The kind of event a step or hook emits.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Level {
    Info,
    Warning,
    Error,
}

impl<'a> Reporting<'a> {
    pub(crate) fn new(emitting: &'a mut dyn Emitting, stamp: impl Into<Stamp>) -> Self {
        Self {
            emitting,
            stamp: stamp.into(),
        }
    }

    pub(crate) fn emit(
        &mut self,
        level: Level,
        message: String,
        data: Option<AnyValue>,
    ) -> Emitted<'_> {
        let body = match self.stamp.clone() {
            Stamp::Step(step) => step_event(level, step, message, data),
            Stamp::Hook(hook) => journey_event(level, hook, message, data),
        };
        self.emitting.relay(body)
    }
}

fn step_event(
    level: Level,
    step: StepAttempt,
    message: String,
    data: Option<AnyValue>,
) -> EventBody {
    match level {
        Level::Info => EventBody::StepInfo {
            step,
            message,
            data,
        },
        Level::Warning => EventBody::StepWarning {
            step,
            message,
            data,
        },
        Level::Error => EventBody::StepError {
            step,
            message,
            data,
        },
    }
}

fn journey_event(
    level: Level,
    hook: HookSource,
    message: String,
    data: Option<AnyValue>,
) -> EventBody {
    match level {
        Level::Info => EventBody::JourneyInfo {
            hook,
            message,
            data,
        },
        Level::Warning => EventBody::JourneyWarning {
            hook,
            message,
            data,
        },
        Level::Error => EventBody::JourneyError {
            hook,
            message,
            data,
        },
    }
}
