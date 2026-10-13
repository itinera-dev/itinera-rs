use rstest::rstest;

use super::*;
use crate::executor::Refusal;
use crate::executor::fixtures::{Execute, Shop, attempt_on, on, run_on, run_tagged_on};
use crate::journey::{Abort, JourneyStatus};
use crate::report::fixtures::entries;
use crate::report::{DefaultDispatcherFactory, Dispatcher, DispatcherFactory};

#[rstest]
#[case::the_default_dispatcher(on::<DefaultDispatcherFactory>)]
#[case::a_dispatcher_that_ignores_failures(on::<CarelessFactory>)]
#[case::a_dispatcher_that_stops_at_any_failure(on::<StrictFactory>)]
fn a_reporter_that_fails_aborts_the_journey_and_only_journey_aborted_reaches_the_others(
    #[case] execute: Execute,
) {
    let (result, log) = run_on(execute, Shop::failing("fragile", "attempt_started"));

    let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = result.status else {
        panic!(
            "the journey was not aborted by a reporter: {:?}",
            result.status
        );
    };
    assert_eq!(error.to_string(), "fragile failed on attempt_started");
    assert_eq!(
        entries(&log),
        [
            "audit journey_id",
            "fragile journey_id",
            "metrics journey_id",
            "audit journey_started",
            "fragile journey_started",
            "metrics journey_started",
            "audit attempt_started",
            "fragile attempt_started",
            "audit journey_aborted",
            "metrics journey_aborted"
        ]
    );
}

#[rstest]
#[case::the_default_dispatcher(on::<DefaultDispatcherFactory>)]
#[case::a_dispatcher_that_ignores_failures(on::<CarelessFactory>)]
#[case::a_dispatcher_that_stops_at_any_failure(on::<StrictFactory>)]
fn a_reporter_that_fails_while_journey_aborted_is_delivered_is_ignored(#[case] execute: Execute) {
    let mut shop = Shop::failing("audit", "attempt_started");
    shop.fails_on.push(("fragile", "journey_aborted"));
    let (result, log) = run_on(execute, shop);

    let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = result.status else {
        panic!(
            "the journey was not aborted by a reporter: {:?}",
            result.status
        );
    };
    assert_eq!(error.to_string(), "audit failed on attempt_started");
    assert_eq!(
        entries(&log),
        [
            "audit journey_id",
            "fragile journey_id",
            "metrics journey_id",
            "audit journey_started",
            "fragile journey_started",
            "metrics journey_started",
            "audit attempt_started",
            "fragile journey_aborted",
            "metrics journey_aborted"
        ]
    );
}

#[rstest]
#[case::the_default_dispatcher(on::<DefaultDispatcherFactory>)]
#[case::a_dispatcher_that_ignores_failures(on::<CarelessFactory>)]
#[case::a_dispatcher_that_stops_at_any_failure(on::<StrictFactory>)]
fn a_reporter_that_fails_on_global_metadata_keeps_it_from_the_reporters_after_it(
    #[case] execute: Execute,
) {
    let (result, log) = attempt_on(execute, Shop::failing("fragile", "journey_id"));

    let Err(Refusal::GlobalMetadata(error)) = result else {
        panic!("the journey was not refused for its global metadata");
    };
    assert_eq!(error.to_string(), "fragile failed on journey_id");
    assert_eq!(entries(&log), ["audit journey_id", "fragile journey_id"]);
}

#[rstest]
#[case::the_default_dispatcher(on::<DefaultDispatcherFactory>)]
#[case::a_dispatcher_that_ignores_failures(on::<CarelessFactory>)]
#[case::a_dispatcher_that_stops_at_any_failure(on::<StrictFactory>)]
fn a_reporter_that_fails_on_an_entry_a_step_sets_aborts_the_journey_and_only_journey_aborted_reaches_the_others(
    #[case] execute: Execute,
) {
    let (result, log) = run_tagged_on(execute, Shop::failing("fragile", "user_id"));

    let JourneyStatus::Aborted(Abort::ReporterFailed(error)) = result.status else {
        panic!(
            "the journey was not aborted by a reporter: {:?}",
            result.status
        );
    };
    assert_eq!(error.to_string(), "fragile failed on user_id");
    assert_eq!(
        entries(&log),
        [
            "audit journey_id",
            "fragile journey_id",
            "metrics journey_id",
            "audit journey_started",
            "fragile journey_started",
            "metrics journey_started",
            "audit attempt_started",
            "fragile attempt_started",
            "metrics attempt_started",
            "audit user_id",
            "fragile user_id",
            "audit journey_aborted",
            "metrics journey_aborted"
        ]
    );
}

/// A dispatcher that ignores the errors of its reporters and delivers every event and entry
/// to all.
#[derive(Default)]
struct Careless {
    reporters: Vec<Box<dyn Reporter>>,
}

impl Dispatcher for Careless {
    fn add(&mut self, reporter: Box<dyn Reporter>) -> Result<(), Error> {
        self.reporters.push(reporter);
        Ok(())
    }

    fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
        for reporter in &mut self.reporters {
            let _ignored = reporter.report(event);
        }
        Ok(())
    }

    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        for reporter in &mut self.reporters {
            let _ignored = reporter.set_global_metadata(key, value);
        }
        Ok(())
    }
}

#[derive(Default)]
struct CarelessFactory;

impl DispatcherFactory for CarelessFactory {
    type Dispatcher = Careless;

    fn create(&mut self) -> Result<Careless, Error> {
        Ok(Careless::default())
    }
}

/// A dispatcher that stops delivering an event or entry at the first reporter that fails,
/// whatever the event.
#[derive(Default)]
struct Strict {
    reporters: Vec<Box<dyn Reporter>>,
}

impl Dispatcher for Strict {
    fn add(&mut self, reporter: Box<dyn Reporter>) -> Result<(), Error> {
        self.reporters.push(reporter);
        Ok(())
    }

    fn dispatch(&mut self, event: &Event) -> Result<(), Error> {
        self.reporters
            .iter_mut()
            .try_for_each(|reporter| reporter.report(event))
    }

    fn set_global_metadata(&mut self, key: &str, value: &AnyValue) -> Result<(), Error> {
        self.reporters
            .iter_mut()
            .try_for_each(|reporter| reporter.set_global_metadata(key, value))
    }
}

#[derive(Default)]
struct StrictFactory;

impl DispatcherFactory for StrictFactory {
    type Dispatcher = Strict;

    fn create(&mut self) -> Result<Strict, Error> {
        Ok(Strict::default())
    }
}
