//! Reading the mistakes in a test from what itinera reported when it stopped the test.

use super::{Mistake, MistakeName};

/// The mistakes what itinera reported names, in order, or none when it names something else.
pub(crate) fn mistakes(reported: &str) -> Option<Vec<Mistake>> {
    match reported.strip_prefix("the script leaves out:\n") {
        Some(left_out) => left_out.lines().map(left_out_one).collect(),
        None => Some(vec![written(reported)?]),
    }
}

/// One line of what a script leaves out, which the check before the journey reports.
fn left_out_one(line: &str) -> Option<Mistake> {
    let line = line.strip_prefix("- ")?;
    if let Some(step) = line.strip_suffix(", since every step must be scripted") {
        return Some(Mistake::of_step(MistakeName::StepLeftOut, step));
    }
    if let Some((rescue, _)) = line.split_once(", since ") {
        return Some(Mistake::of_step(MistakeName::RescueStepLeftOut, rescue));
    }
    let (attempt, step) = line.strip_prefix("attempt ")?.split_once(" of ")?;
    Mistake::of_attempt(MistakeName::AttemptLeftOut, step, attempt)
}

/// A mistake reported as the test writes it.
fn written(reported: &str) -> Option<Mistake> {
    [
        unknown_name,
        gap,
        attempt_scripted_twice,
        no_such_attempt,
        scripted_twice,
        rescue_skips,
        undeclared_handle,
        another_service,
    ]
    .into_iter()
    .find_map(|reading| reading(reported))
}

/// A step or a provider the workflow does not have, which the record of a scripted run reports
/// without naming the workflow.
fn unknown_name(reported: &str) -> Option<Mistake> {
    let after = reported.strip_prefix("the workflow ")?;
    let named = after
        .strip_prefix("has no ")
        .or_else(|| after_the_workflows_name(after))?;
    match named.split_once(" named ")? {
        ("step", step) => Some(Mistake::of_step(MistakeName::UnknownStep, step)),
        ("service provider", provider) => {
            Some(Mistake::of_provider(MistakeName::UnknownProvider, provider))
        }
        _ => None,
    }
}

/// What follows " has no " after the workflow's name.
fn after_the_workflows_name(named: &str) -> Option<&str> {
    let (_, after) = named.split_once(" has no ")?;
    Some(after)
}

fn gap(reported: &str) -> Option<Mistake> {
    let (_, rest) = reported.strip_prefix("attempt ")?.split_once(" of ")?;
    let (step, attempt) = rest.split_once(" is missing before attempt ")?;
    Mistake::of_attempt(MistakeName::GapInTheAttempts, step, attempt)
}

/// An attempt scripted already, by number or with the remaining attempts, with or without a hint
/// after it.
fn attempt_scripted_twice(reported: &str) -> Option<Mistake> {
    let (attempt, rest) = reported.strip_prefix("attempt ")?.split_once(" of ")?;
    let step = rest
        .strip_suffix(" is scripted already")
        .or_else(|| before_the_hint(rest))?;
    Mistake::of_attempt(MistakeName::AttemptScriptedTwice, step, attempt)
}

/// The step an attempt scripted already belongs to, when a hint follows.
fn before_the_hint(rest: &str) -> Option<&str> {
    let (step, _) = rest.split_once(" is scripted already; ")?;
    Some(step)
}

/// An attempt the step can never have.
fn no_such_attempt(reported: &str) -> Option<Mistake> {
    let (step, rest) = reported.split_once(" has no attempt ")?;
    let (attempt, _) = rest.split_once("; ")?;
    Mistake::of_attempt(MistakeName::NoSuchAttempt, step, attempt)
}

/// A step named twice; an attempt scripted twice is another mistake.
fn scripted_twice(reported: &str) -> Option<Mistake> {
    let step = reported.strip_suffix(" is scripted already")?;
    (!step.contains(' ')).then(|| Mistake::of_step(MistakeName::StepScriptedTwice, step))
}

fn rescue_skips(reported: &str) -> Option<Mistake> {
    let step = reported.strip_suffix(" is a rescue step, which cannot skip")?;
    Some(Mistake::of_step(MistakeName::RescueStepSkips, step))
}

fn undeclared_handle(reported: &str) -> Option<Mistake> {
    let (step, rest) = reported.split_once(" declares no ")?;
    match rest.split_once(", so it cannot ")? {
        ("contributor", _) => Some(Mistake::of_step(MistakeName::NoContributor, step)),
        ("reporter", _) => Some(Mistake::of_step(MistakeName::NoReporter, step)),
        _ => None,
    }
}

fn another_service(reported: &str) -> Option<Mistake> {
    let (provider, _) = reported
        .strip_prefix("the fake of the service provider ")?
        .split_once(" supplies ")?;
    Some(Mistake::of_provider(MistakeName::AnotherService, provider))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn what_a_script_leaves_out_names_each_mistake() {
        let reported = "the script leaves out:\n- attempt 2 of charge\n- charge_rescue, since \
                        charge can fail on attempt 1\n- ship, since every step must be scripted";
        assert_eq!(
            mistakes(reported),
            Some(vec![
                Mistake {
                    attempt: Some(2),
                    ..Mistake::of_step(MistakeName::AttemptLeftOut, "charge")
                },
                Mistake::of_step(MistakeName::RescueStepLeftOut, "charge_rescue"),
                Mistake::of_step(MistakeName::StepLeftOut, "ship"),
            ])
        );
    }

    #[rstest]
    #[case::unknown_step(
        "the workflow orders has no step named refund",
        MistakeName::UnknownStep,
        "refund"
    )]
    #[case::unknown_step_in_the_record(
        "the workflow has no step named refund",
        MistakeName::UnknownStep,
        "refund"
    )]
    #[case::step_scripted_twice(
        "charge is scripted already",
        MistakeName::StepScriptedTwice,
        "charge"
    )]
    #[case::rescue_step_skips(
        "charge_rescue is a rescue step, which cannot skip",
        MistakeName::RescueStepSkips,
        "charge_rescue"
    )]
    #[case::no_contributor(
        "charge declares no contributor, so it cannot contribute",
        MistakeName::NoContributor,
        "charge"
    )]
    #[case::no_reporter(
        "charge declares no reporter, so it cannot set global metadata",
        MistakeName::NoReporter,
        "charge"
    )]
    fn a_mistake_in_writing_the_script_names_its_step(
        #[case] reported: &str,
        #[case] mistake: MistakeName,
        #[case] step: &str,
    ) {
        assert_eq!(
            mistakes(reported),
            Some(vec![Mistake::of_step(mistake, step)])
        );
    }

    #[test]
    fn a_gap_names_the_attempt_written_too_early() {
        assert_eq!(
            mistakes("attempt 2 of charge is missing before attempt 3"),
            Some(vec![Mistake {
                attempt: Some(3),
                ..Mistake::of_step(MistakeName::GapInTheAttempts, "charge")
            }])
        );
    }

    #[rstest]
    #[case::scripted_by_number(
        "attempt 1 of charge is scripted already",
        MistakeName::AttemptScriptedTwice,
        1
    )]
    #[case::scripted_with_a_hint(
        "attempt 1 of charge is scripted already; on_remaining_attempts() covers the rest",
        MistakeName::AttemptScriptedTwice,
        1
    )]
    #[case::past_the_retry_budget(
        "charge has no attempt 3; it can have at most 2",
        MistakeName::NoSuchAttempt,
        3
    )]
    #[case::attempt_zero(
        "charge has no attempt 0; attempts are numbered from 1",
        MistakeName::NoSuchAttempt,
        0
    )]
    fn a_mistake_in_choosing_attempts_names_its_step_and_attempt(
        #[case] reported: &str,
        #[case] mistake: MistakeName,
        #[case] attempt: u32,
    ) {
        assert_eq!(
            mistakes(reported),
            Some(vec![Mistake {
                attempt: Some(attempt),
                ..Mistake::of_step(mistake, "charge")
            }])
        );
    }

    #[rstest]
    #[case::unknown_provider(
        "the workflow orders has no service provider named paypal",
        MistakeName::UnknownProvider,
        "paypal"
    )]
    #[case::another_service(
        "the fake of the service provider payments supplies Meter, but the provider supplies \
         Labeller",
        MistakeName::AnotherService,
        "payments"
    )]
    fn a_mistake_in_a_fake_names_its_provider(
        #[case] reported: &str,
        #[case] mistake: MistakeName,
        #[case] provider: &str,
    ) {
        assert_eq!(
            mistakes(reported),
            Some(vec![Mistake::of_provider(mistake, provider)])
        );
    }

    #[rstest]
    #[case::a_bug("called `Option::unwrap()` on a `None` value")]
    fn anything_else_names_no_mistake(#[case] reported: &str) {
        assert_eq!(mistakes(reported), None);
    }
}
