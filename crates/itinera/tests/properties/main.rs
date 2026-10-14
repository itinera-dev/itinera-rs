//! Properties of every journey's event stream, checked over generated workflows: random
//! outcomes, retry budgets and lifecycles, hooks that fail and a reporter that may fail. The
//! workflows have no rescue steps and no services.

// Marked as test code, where the workspace's lints allow `unwrap`.
#[cfg(test)]
mod journey;
#[cfg(test)]
mod plan;
#[cfg(test)]
mod scripted;
#[cfg(test)]
mod stream;

use itinera::event::Event;
use itinera::journey::StatusKind;
use itinera::step::StepAttempt;
use proptest::prelude::{prop_assert, prop_assert_eq, proptest};
use proptest::test_runner::TestCaseError;

#[cfg(feature = "async")]
use journey::run_async;
use journey::{BAG_VALUE, Journey, run, steps};
use plan::plans;
#[cfg(feature = "async")]
use stream::untimed;
use stream::{
    attempt_numbers, attempts, counted_from_one, ended_status, ends_journey, is_engine_event,
    needed_decisions, outcome_facts, sequence, started, starts_journey, step_decisions,
    succeeds_or_fails,
};

/// What an attempt's events hold of one kind, such as its outcome facts.
type Found = fn(&[Event]) -> Vec<&StepAttempt>;

/// Checks that every attempt of the journey holds what it needs: the attempt during which the
/// journey was aborted, which is the last, may hold nothing instead.
fn every_attempt_holds(
    journey: &Journey,
    found: Found,
    needed: Found,
) -> Result<(), TestCaseError> {
    let attempts: Vec<&[Event]> = attempts(&journey.events).collect();
    let Some((last, others)) = attempts.split_last() else {
        return Ok(());
    };
    for attempt in others {
        prop_assert_eq!(found(attempt), needed(attempt));
    }
    let cut_short = journey.status == StatusKind::Aborted && found(last).is_empty();
    prop_assert!(
        cut_short || found(last) == needed(last),
        "the last attempt holds {:?} instead of {:?}",
        found(last),
        needed(last)
    );
    Ok(())
}

proptest! {
    #[test]
    fn a_journey_starts_with_journey_started_and_ends_with_the_event_of_its_status(
        plan in plans()
    ) {
        let journey = run(&plan);
        let endings: Vec<StatusKind> = journey.events.iter().filter_map(ended_status).collect();
        let trailing = journey.events.iter().rev().take_while(|event| ends_journey(event)).count();
        prop_assert_eq!(journey.events.first().map(Event::kind), Some("journey_started"));
        prop_assert_eq!(journey.events.iter().filter(|event| starts_journey(event)).count(), 1);
        prop_assert_eq!(journey.events.last().and_then(ended_status), Some(journey.status));
        prop_assert_eq!(trailing, endings.len());
        // A reporter that fails on `journey_succeeded` or `journey_failed` aborts the journey
        // after it, and nothing else does. The flaky reporter, added after the recorder, receives
        // the recorder's events up to the one it fails on.
        let failed_on_ending =
            journey.events.iter().position(succeeds_or_fails).and_then(counted_from_one);
        prop_assert_eq!(
            endings.len() == 2,
            failed_on_ending.is_some() && plan.reporter_fails_on == failed_on_ending
        );
        prop_assert!(
            matches!(
                endings.as_slice(),
                [_] | [StatusKind::Succeeded | StatusKind::Failed, StatusKind::Aborted]
            ),
            "{:?}",
            endings
        );
    }

    #[test]
    fn sequence_numbers_increase_from_one(plan in plans()) {
        let sequences: Vec<u64> = run(&plan).events.iter().map(sequence).collect();
        prop_assert_eq!(sequences.first(), Some(&1));
        prop_assert!(sequences.is_sorted_by(u64::lt), "{:?}", sequences);
    }

    #[test]
    fn every_attempt_has_exactly_one_outcome_fact_unless_the_journey_is_aborted_during_it(
        plan in plans()
    ) {
        every_attempt_holds(&run(&plan), outcome_facts, started)?;
    }

    #[test]
    fn exactly_one_step_decision_follows_each_failed_attempt_unless_the_journey_is_aborted_first(
        plan in plans()
    ) {
        every_attempt_holds(&run(&plan), step_decisions, needed_decisions)?;
    }

    #[test]
    fn a_step_without_a_rescue_step_is_attempted_at_most_its_retry_budget_plus_one_times_counting_from_one(
        plan in plans()
    ) {
        let journey = run(&plan);
        for (step, planned) in steps(&plan) {
            let numbers = attempt_numbers(&journey.events, step);
            let counted: Vec<u32> = (1..).take(numbers.len()).collect();
            prop_assert!(numbers.len() <= usize::from(planned.retry_budget) + 1);
            prop_assert_eq!(numbers, counted);
        }
    }

    #[test]
    fn engine_events_carry_no_values_of_the_data_bag(plan in plans()) {
        let journey = run(&plan);
        for event in journey.events.iter().filter(|event| is_engine_event(event)) {
            let written = format!("{:?}", event.body);
            prop_assert!(!written.contains(BAG_VALUE), "{}", written);
            // No generated reason has details, so a value written here could only be data.
            prop_assert!(!written.contains("AnyValue"), "{}", written);
        }
    }

    #[cfg(feature = "async")]
    #[test]
    fn both_executors_emit_the_same_stream_apart_from_journey_ids_and_timestamps(
        plan in plans()
    ) {
        let local: Vec<String> = run(&plan).events.iter().map(untimed).collect();
        let asynchronous: Vec<String> = run_async(&plan).events.iter().map(untimed).collect();
        prop_assert_eq!(local, asynchronous);
    }
}
