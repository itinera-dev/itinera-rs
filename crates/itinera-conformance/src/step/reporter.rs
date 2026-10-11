//! A step's reporter, of either mode, as the script uses it.

use std::future::{Future, ready};
use std::pin::Pin;

use itinera::error::Interrupted;
use itinera::step::{AsyncStepReporter, StepReporter};
use itinera::value::Value as Storable;

use crate::model::Level;
use crate::value::{Takes, Typed};

/// What emitting an event gives, once awaited.
pub(crate) type Emitted<'r> = Pin<Box<dyn Future<Output = Result<(), Interrupted>> + Send + 'r>>;

/// A step's reporter, of either mode, as the script uses it.
pub(crate) trait Emits: Send {
    fn emit(&mut self, level: Level, message: String, data: Option<Typed>) -> Emitted<'_>;
}

impl Emits for StepReporter<'_> {
    fn emit(&mut self, level: Level, message: String, data: Option<Typed>) -> Emitted<'_> {
        let emitted = match data {
            None => match level {
                Level::Info => self.info(message),
                Level::Warning => self.warning(message),
                Level::Error => self.error(message),
            },
            Some(data) => data.given_to(Reporting {
                reporter: self,
                level,
                message,
            }),
        };
        Box::pin(ready(emitted))
    }
}

/// Emits an event with data from a synchronous step.
struct Reporting<'r, 'a> {
    reporter: &'r mut StepReporter<'a>,
    level: Level,
    message: String,
}

impl Takes for Reporting<'_, '_> {
    type Output = Result<(), Interrupted>;

    fn take<T: Storable>(self, data: T) -> Result<(), Interrupted> {
        match self.level {
            Level::Info => self.reporter.info_with(self.message, data),
            Level::Warning => self.reporter.warning_with(self.message, data),
            Level::Error => self.reporter.error_with(self.message, data),
        }
    }
}

impl Emits for AsyncStepReporter<'_> {
    fn emit(&mut self, level: Level, message: String, data: Option<Typed>) -> Emitted<'_> {
        match data {
            None => match level {
                Level::Info => Box::pin(self.info(message)),
                Level::Warning => Box::pin(self.warning(message)),
                Level::Error => Box::pin(self.error(message)),
            },
            Some(data) => data.given_to(AsyncReporting {
                reporter: self,
                level,
                message,
            }),
        }
    }
}

/// Emits an event with data from an asynchronous step.
struct AsyncReporting<'r, 'a> {
    reporter: &'r mut AsyncStepReporter<'a>,
    level: Level,
    message: String,
}

impl<'r> Takes for AsyncReporting<'r, '_> {
    type Output = Emitted<'r>;

    fn take<T: Storable>(self, data: T) -> Emitted<'r> {
        match self.level {
            Level::Info => Box::pin(self.reporter.info_with(self.message, data)),
            Level::Warning => Box::pin(self.reporter.warning_with(self.message, data)),
            Level::Error => Box::pin(self.reporter.error_with(self.message, data)),
        }
    }
}
