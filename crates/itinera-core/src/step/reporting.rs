//! What step and hook reporters of either mode emit through and set global metadata through: the
//! journey, with who emits and the kind of event, or a recorder in a test.

use super::StepAttempt;
use crate::engine::{Emitting, Relayed};
#[cfg(any(test, feature = "testing"))]
use crate::event::Recorded;
use crate::event::{EventBody, HookSource, Level, Source};
use crate::value::AnyValue;

/// What a step or hook reporter of either mode emits through: the journey, while the attempt or
/// the hook runs, or a test's recorder.
#[derive(derive_more::Debug)]
pub(crate) enum Reporting<'a> {
    /// The journey, which stamps every event and entry of global metadata with its source.
    Journey {
        #[debug(skip)]
        emitting: &'a mut dyn Emitting,
        source: Source,
    },
    /// A test's recorder.
    #[cfg(any(test, feature = "testing"))]
    Recorded(&'a mut Recorded),
}

impl<'a> Reporting<'a> {
    /// A reporter that emits for `source`, which every event it emits carries, as does every
    /// entry of global metadata it sets.
    pub(crate) fn new(emitting: &'a mut dyn Emitting, source: impl Into<Source>) -> Self {
        Self::Journey {
            emitting,
            source: source.into(),
        }
    }

    pub(crate) fn emit(
        &mut self,
        level: Level,
        message: String,
        data: Option<AnyValue>,
    ) -> Relayed<'_> {
        match self {
            Self::Journey { emitting, source } => {
                let body = match source.clone() {
                    Source::Step(step) => step_event(level, step, message, data),
                    Source::Hook(hook) => journey_event(level, hook, message, data),
                };
                emitting.relay(body)
            }
            #[cfg(any(test, feature = "testing"))]
            Self::Recorded(recorded) => recorded.record(level, message, data),
        }
    }

    pub(crate) fn set_global_metadata(&mut self, key: String, value: AnyValue) -> Relayed<'_> {
        match self {
            Self::Journey { emitting, source } => {
                emitting.set_global_metadata(source.clone(), key, value)
            }
            #[cfg(any(test, feature = "testing"))]
            Self::Recorded(recorded) => recorded.set(key, value),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::fixtures::{CHARGE, kinds, scripted, travel_workflow};
    use crate::step::Outcome;
    use crate::workflow::fixtures::orders;

    #[test]
    fn a_step_emits_its_own_events_stamped_with_its_attempt_before_its_outcome() {
        let workflow = orders().step(scripted(|_, reporter| {
            reporter.info("charging")?;
            reporter.warning_with("slow", 300_i64)?;
            reporter.error("no receipt")?;
            Ok(Outcome::success())
        }));

        let (_, events) = travel_workflow(workflow);

        assert_eq!(
            kinds(&events),
            [
                "journey_started",
                "attempt_started",
                "step_info",
                "step_warning",
                "step_error",
                "step_succeeded",
                "journey_succeeded"
            ]
        );
        let Some(EventBody::StepWarning {
            step,
            message,
            data,
        }) = events.get(3).map(|e| &e.body)
        else {
            panic!("the fourth event is not step_warning");
        };
        assert_eq!(step, &StepAttempt::first(CHARGE));
        assert_eq!(message, "slow");
        assert_eq!(
            data.as_ref().and_then(AnyValue::downcast_ref),
            Some(&300_i64)
        );
    }
}
