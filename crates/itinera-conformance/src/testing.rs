//! Itinera's test support, as the scenario uses it: its recording reporter, its fakes, its
//! scripted runs and its recorders, and the mistakes in a test that stop it.

mod mistakes;
mod script;

use std::any::Any;
use std::collections::BTreeMap;
use std::future::Future;
use std::panic::{self, AssertUnwindSafe};

use futures::FutureExt as _;
use itinera::error::Error;
use itinera::event::Level as EventLevel;
use itinera::journey::Contributions;
use itinera::policy::HookEventReporter;
use itinera::report::RecordingReporter;
use itinera::step::StepEventReporter;

pub(crate) use mistakes::{Mistake, MistakeName, mistakes};
pub(crate) use script::{PlaysScripts, written};

use crate::model::{Fake, Level};
use crate::service::FakeCalls;
use itinera::workflow::Supplied;

/// What the scenario's use of test support gave.
#[derive(Debug, Default)]
pub(crate) struct Tested {
    /// The recording reporter of test support the instance was given, if it was given one.
    pub(crate) recording_reporter: Option<RecordingReporter>,
    /// The calls of each fake, by the name of the provider it replaces.
    pub(crate) fake_calls: BTreeMap<String, FakeCalls>,
    /// What test support reported when it stopped the test before the journey.
    pub(crate) stopped: Option<String>,
    pub(crate) contributions: Contributions,
    pub(crate) step_events: StepEventReporter,
    pub(crate) hook_events: HookEventReporter,
    /// What an input adapter called alone returned.
    pub(crate) adapter_returned: Option<Result<Supplied, Error>>,
}

impl Tested {
    /// New counts of the calls of these fakes, in order, which the scenario reads by the name of
    /// the provider each replaces.
    pub(crate) fn counting(&mut self, fakes: &[Fake]) -> Vec<FakeCalls> {
        fakes
            .iter()
            .map(|fake| self.counted(&fake.provider))
            .collect()
    }

    fn counted(&mut self, provider: &str) -> FakeCalls {
        let calls = FakeCalls::default();
        self.fake_calls.insert(provider.to_owned(), calls.clone());
        calls
    }
}

/// The level itinera gives an event of this level.
pub(crate) fn event_level(level: Level) -> EventLevel {
    match level {
        Level::Info => EventLevel::Info,
        Level::Warning => EventLevel::Warning,
        Level::Error => EventLevel::Error,
    }
}

/// Runs what test support may stop, as it stops a test with a mistake in it: with a panic,
/// as Rust reports a misuse in a declaration. Gives what it returned, or what the panic said.
pub(crate) async fn unless_stopped<T>(run: impl Future<Output = T>) -> Result<T, String> {
    AssertUnwindSafe(run).catch_unwind().await.map_err(said)
}

/// Reads what test support may stop, as [`unless_stopped`] runs it.
pub(crate) fn unless_stopped_reading<T>(read: impl FnOnce() -> T) -> Result<T, String> {
    panic::catch_unwind(AssertUnwindSafe(read)).map_err(said)
}

/// What a panic said.
fn said(payload: Box<dyn Any + Send>) -> String {
    match payload.downcast::<String>() {
        Ok(message) => *message,
        Err(payload) => payload.downcast_ref::<&str>().map_or_else(
            || "a panic without a message".to_owned(),
            ToString::to_string,
        ),
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use super::*;

    #[test]
    fn a_stopped_test_gives_what_its_panic_said() {
        let stopped = block_on(unless_stopped(async {
            panic!("charge is scripted already")
        }));
        assert_eq!(
            stopped,
            Err::<(), _>("charge is scripted already".to_owned())
        );
    }

    #[test]
    fn a_test_that_is_not_stopped_gives_what_it_returned() {
        assert_eq!(block_on(unless_stopped(async { 40 })), Ok(40));
    }
}
