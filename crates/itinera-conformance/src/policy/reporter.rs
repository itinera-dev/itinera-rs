//! A hook's reporter, of either mode, as the scripts use it.

use std::future::ready;

use itinera::policy::{AsyncHookReporter, HookReporter};

use crate::model::Level;
use crate::step::Emitted;

/// A hook's reporter, of either mode, as the scripts use it.
pub(super) trait Emits: Send {
    fn emit(&mut self, level: Level, message: String) -> Emitted<'_>;
}

impl Emits for HookReporter<'_> {
    fn emit(&mut self, level: Level, message: String) -> Emitted<'_> {
        let emitted = match level {
            Level::Info => self.info(message),
            Level::Warning => self.warning(message),
            Level::Error => self.error(message),
        };
        Box::pin(ready(emitted))
    }
}

impl Emits for AsyncHookReporter<'_> {
    fn emit(&mut self, level: Level, message: String) -> Emitted<'_> {
        match level {
            Level::Info => Box::pin(self.info(message)),
            Level::Warning => Box::pin(self.warning(message)),
            Level::Error => Box::pin(self.error(message)),
        }
    }
}
