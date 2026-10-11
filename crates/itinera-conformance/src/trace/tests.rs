use rstest::rstest;
use serde_json::json;

use super::*;

fn line(event: &str, cells: &[(&'static str, &str)]) -> Line {
    Line {
        event: event.to_owned(),
        cells: cells.iter().map(owned).collect(),
        data: None,
        carried: Vec::new(),
    }
}

fn owned(&(column, cell): &(&'static str, &str)) -> (&'static str, String) {
    (column, cell.to_owned())
}

fn stream() -> Vec<Line> {
    vec![
        line("journey_started", &[]),
        line("attempt_started", &[("step", "charge"), ("attempt", "1")]),
        line("step_succeeded", &[("step", "charge"), ("attempt", "1")]),
        line("journey_succeeded", &[("decided by", "default")]),
    ]
}

#[test]
fn a_table_naming_an_event_outside_the_catalogue_is_in_error() {
    let row = Row::from_iter([("event".to_owned(), "step_started".to_owned())]);
    assert_eq!(
        Line::expected(&row).unwrap_err(),
        ModelError::UnknownEvent("step_started".to_owned())
    );
}

#[rstest]
#[case::no_cells("step_failed", &[], true)]
#[case::a_cell_that_matches("step_failed", &[("code", "declined")], true)]
#[case::a_cell_that_differs("step_failed", &[("code", "timeout")], false)]
#[case::a_cell_the_event_leaves_empty("step_failed", &[("retriable", "true")], false)]
#[case::another_event("step_skipped", &[], false)]
fn an_empty_cell_is_not_compared(
    #[case] kind: &str,
    #[case] cells: &[(&'static str, &str)],
    #[case] matches: bool,
) {
    let event = line("step_failed", &[("step", "charge"), ("code", "declined")]);
    assert_eq!(line(kind, cells).matches(&event), matches);
}

#[rstest]
#[case::an_abort_without_an_adapter(&[("step", "charge"), ("key", "amount")], true)]
#[case::an_abort_naming_an_adapter(&[("step", "charge"), ("adapter", "pricing")], false)]
fn only_an_event_without_an_adapter_names_no_adapter(
    #[case] cells: &[(&'static str, &str)],
    #[case] names_no_adapter: bool,
) {
    assert_eq!(
        line("journey_aborted", cells).names_no_adapter(),
        names_no_adapter
    );
}

#[rstest]
#[case::the_same_object_written_in_another_order(r#"{"slow": true, "ms": 1200}"#, true)]
#[case::another_object(r#"{"ms": 1300}"#, false)]
fn data_is_compared_as_json(#[case] data: &str, #[case] matches: bool) {
    let mut event = line("step_warning", &[]);
    event.data = Some(json!({"ms": 1200, "slow": true}));
    let mut expected = line("step_warning", &[]);
    expected.data = Some(json(data).unwrap());
    assert_eq!(expected.matches(&event), matches);
}

#[rstest]
#[case::within_an_error_as_text(json!("4111"), true)]
#[case::within_an_error_as_a_number(json!(4111), true)]
#[case::within_a_value(json!(4112), true)]
#[case::as_part_of_a_value(json!({"number": 4112}), true)]
#[case::text_it_does_not_carry(json!("secret-token"), false)]
#[case::a_number_it_does_not_carry(json!(4113), false)]
fn a_value_is_carried_within_any_text_or_value_an_event_carries(
    #[case] value: Value,
    #[case] carried: bool,
) {
    let mut event = line("journey_aborted", &[("error", "card 4111 declined")]);
    event.carried.push(json!({"card": {"number": 4112}}));
    assert_eq!(event.carries_value(&value), carried);
}

#[rstest]
#[case::the_step(json!("charge"))]
#[case::the_attempt(json!(1))]
#[case::who_decided(json!("default"))]
#[case::an_empty_text(json!(""))]
fn the_names_and_labels_an_event_gives_are_not_values_it_carries(#[case] value: Value) {
    let event = line(
        "step_retrying",
        &[
            ("step", "charge"),
            ("attempt", "1"),
            ("decided by", "default"),
        ],
    );
    assert!(!event.carries_value(&value));
}

#[rstest]
#[case::as_its_message("first", true)]
#[case::within_what_it_carries("later", true)]
#[case::not_at_all("second", false)]
fn a_message_is_carried_as_a_message_an_error_or_a_reasons_message(
    #[case] message: &str,
    #[case] carried: bool,
) {
    let mut event = line("step_info", &[("message", "first")]);
    event.carried.push(json!("later"));
    assert_eq!(event.carries_message(message), carried);
}

fn lines(kinds: &[&str]) -> Vec<Line> {
    kinds.iter().map(|kind| line(kind, &[])).collect()
}

#[rstest]
#[case::with_others_in_between(&["journey_started", "journey_succeeded"], true)]
#[case::in_another_order(&["journey_succeeded", "journey_started"], false)]
#[case::an_event_the_stream_lacks(&["step_failed"], false)]
fn included_events_may_have_others_in_between_but_keep_their_order(
    #[case] kinds: &[&str],
    #[case] included: bool,
) {
    assert_eq!(includes_in_order(&stream(), &lines(kinds)), included);
}

#[rstest]
#[case::the_whole_stream(
    &["journey_started", "attempt_started", "step_succeeded", "journey_succeeded"],
    true
)]
#[case::the_stream_without_its_first_event(
    &["attempt_started", "step_succeeded", "journey_succeeded"],
    false
)]
fn exact_events_are_the_whole_stream_in_order(#[case] kinds: &[&str], #[case] exact: bool) {
    assert_eq!(are_exactly(&stream(), &lines(kinds)), exact);
}
