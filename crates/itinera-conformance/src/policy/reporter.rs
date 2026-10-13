//! A hook's reporter, of either mode, as the scripts use it.

use std::future::ready;

use itinera::error::Interrupted;
use itinera::policy::{AsyncHookReporter, HookReporter};
use itinera::value::Value as Storable;

use crate::model::Level;
use crate::step::Emitted;
use crate::value::{Takes, Typed};

/// A hook's reporter, of either mode, as the scripts use it.
pub(crate) trait Emits: Send {
    fn emit(&mut self, level: Level, message: String) -> Emitted<'_>;

    fn set_global_metadata(&mut self, key: String, value: Typed) -> Emitted<'_>;
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

    fn set_global_metadata(&mut self, key: String, value: Typed) -> Emitted<'_> {
        Box::pin(ready(value.given_to(Setting {
            reporter: self,
            key,
        })))
    }
}

/// Sets an entry of global metadata from a synchronous hook or service provider.
struct Setting<'r, 'a> {
    reporter: &'r mut HookReporter<'a>,
    key: String,
}

impl Takes for Setting<'_, '_> {
    type Output = Result<(), Interrupted>;

    fn take<T: Storable>(self, value: T) -> Result<(), Interrupted> {
        self.reporter.set_global_metadata(self.key, value)
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

    fn set_global_metadata(&mut self, key: String, value: Typed) -> Emitted<'_> {
        value.given_to(AsyncSetting {
            reporter: self,
            key,
        })
    }
}

/// Sets an entry of global metadata from an asynchronous hook or service provider.
struct AsyncSetting<'r, 'a> {
    reporter: &'r mut AsyncHookReporter<'a>,
    key: String,
}

impl<'r> Takes for AsyncSetting<'r, '_> {
    type Output = Emitted<'r>;

    fn take<T: Storable>(self, value: T) -> Emitted<'r> {
        Box::pin(self.reporter.set_global_metadata(self.key, value))
    }
}
