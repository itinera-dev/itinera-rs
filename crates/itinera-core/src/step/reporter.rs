use super::StepAttempt;
use crate::engine::{Emitted, Emitting, finish};
use crate::error::Interrupted;
use crate::event::{EventBody, HookSource};
use crate::value::{AnyValue, Value};

/// A step's means of emitting its own events, `step_info`, `step_warning` and `step_error`, each
/// stamped with the step and its attempt.
///
/// It is made for one attempt and borrows it, so it cannot outlive it. An event is delivered to
/// every reporter before the call returns. If a reporter fails meanwhile, the journey is aborted
/// with `reporter failed`, and the call returns [`Interrupted`], which the step propagates with
/// `?`: the abort stands whatever the step does afterwards. Data is a value, captured when the
/// event is emitted.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::step::{Outcome, Resolved, Step, StepFactory, StepNeeds, StepReporter};
///
/// struct Charge<'a> {
///     reporter: StepReporter<'a>,
/// }
///
/// impl Step for Charge<'_> {
///     fn run(mut self) -> Result<Outcome, Error> {
///         self.reporter.info_with("charging", 42_i64)?;
///         Ok(Outcome::success())
///     }
/// }
///
/// struct ChargeFactory;
///
/// impl StepFactory for ChargeFactory {
///     type Step<'a> = Charge<'a>;
///
///     fn needs(&self) -> StepNeeds {
///         StepNeeds::new().reporter()
///     }
///
///     fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charge<'a>, Error> {
///         Ok(Charge {
///             reporter: got.reporter()?,
///         })
///     }
/// }
/// ```
#[derive(Debug, derive_more::From)]
pub struct StepReporter<'a> {
    reporting: Reporting<'a>,
}

impl StepReporter<'_> {
    /// Emits `step_info` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::StepReporter;
    ///
    /// fn starting(reporter: &mut StepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.info("starting")
    /// }
    /// ```
    pub fn info(&mut self, message: impl Into<String>) -> Result<(), Interrupted> {
        finish(self.reporting.emit(Level::Info, message.into(), None))
    }

    /// Emits `step_info` with a message and data.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::StepReporter;
    ///
    /// fn charging(reporter: &mut StepReporter<'_>, amount: i64) -> Result<(), Interrupted> {
    ///     reporter.info_with("charging", amount)
    /// }
    /// ```
    pub fn info_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> Result<(), Interrupted> {
        finish(
            self.reporting
                .emit(Level::Info, message.into(), Some(AnyValue::new(data))),
        )
    }

    /// Emits `step_warning` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::StepReporter;
    ///
    /// fn slow(reporter: &mut StepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.warning("the gateway is slow")
    /// }
    /// ```
    pub fn warning(&mut self, message: impl Into<String>) -> Result<(), Interrupted> {
        finish(self.reporting.emit(Level::Warning, message.into(), None))
    }

    /// Emits `step_warning` with a message and data.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::StepReporter;
    ///
    /// fn slow(reporter: &mut StepReporter<'_>, millis: u64) -> Result<(), Interrupted> {
    ///     reporter.warning_with("the gateway is slow", millis)
    /// }
    /// ```
    pub fn warning_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> Result<(), Interrupted> {
        finish(
            self.reporting
                .emit(Level::Warning, message.into(), Some(AnyValue::new(data))),
        )
    }

    /// Emits `step_error` with a message. It does not change the step's outcome.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::StepReporter;
    ///
    /// fn no_receipt(reporter: &mut StepReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.error("the receipt was not printed")
    /// }
    /// ```
    pub fn error(&mut self, message: impl Into<String>) -> Result<(), Interrupted> {
        finish(self.reporting.emit(Level::Error, message.into(), None))
    }

    /// Emits `step_error` with a message and data. It does not change the step's outcome.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the attempt.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::step::StepReporter;
    ///
    /// fn no_receipt(reporter: &mut StepReporter<'_>, printer: String) -> Result<(), Interrupted> {
    ///     reporter.error_with("the receipt was not printed", printer)
    /// }
    /// ```
    pub fn error_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> Result<(), Interrupted> {
        finish(
            self.reporting
                .emit(Level::Error, message.into(), Some(AnyValue::new(data))),
        )
    }
}

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
