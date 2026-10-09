//! The event stream as the cases' event tables describe it.

use std::collections::BTreeMap;

use itinera::event::{
    Event, EventBody, GiveUpCause, HookSource, JourneyAbort, JourneyFailure, MissingData,
    RequestSource, Requester, Source,
};
use itinera::journey::LastFailure;
use itinera::step::{Reason, StepAttempt};
use serde_json::Value;

use crate::model::{EventKind, ModelError, Row, json};

/// The columns an event table may have besides `event`.
const COLUMNS: [&str; 15] = [
    "step",
    "attempt",
    "key",
    "code",
    "retriable",
    "policy",
    "hook",
    "lifecycle",
    "adapter",
    "source",
    "cause",
    "decided by",
    "message",
    "data",
    "error",
];

/// What an event table can say of one event: its kind, and its other columns as text, except
/// `data`, which is compared as JSON.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Line {
    pub(crate) event: String,
    cells: BTreeMap<&'static str, String>,
    data: Option<Value>,
    /// What else the event carries that no column shows: reasons' messages and details, and
    /// the message of an abnormal termination.
    carried: Vec<Value>,
}

impl Line {
    /// Reads one row of an event table, keeping only its non-empty cells.
    pub(crate) fn expected(row: &Row) -> Result<Self, ModelError> {
        let mut line = Self {
            event: EventKind::named(row.required("event")?)?.name().to_owned(),
            ..Self::default()
        };
        if let Some(column) = row.columns().find(is_unknown_column) {
            return Err(ModelError::UnknownColumn(column.to_owned()));
        }
        for column in COLUMNS {
            match (column, row.optional(column)) {
                ("data", Some(data)) => line.data = Some(json(data)?),
                (_, Some(cell)) => {
                    line.cells.insert(column, cell.to_owned());
                }
                (_, None) => {}
            }
        }
        Ok(line)
    }

    /// Whether an event matches every cell this line states.
    pub(crate) fn matches(&self, event: &Self) -> bool {
        self.event == event.event
            && self
                .cells
                .iter()
                .all(|(column, cell)| event.has_cell(column, cell))
            && self.data.as_ref().is_none_or(|data| event.has_data(data))
    }

    /// Whether the event has this text in this column.
    fn has_cell(&self, column: &str, cell: &str) -> bool {
        self.cells.get(column).map(String::as_str) == Some(cell)
    }

    /// Whether the event's data is this JSON.
    fn has_data(&self, data: &Value) -> bool {
        self.data.as_ref() == Some(data)
    }

    fn set(&mut self, column: &'static str, cell: impl ToString) {
        self.cells.insert(column, cell.to_string());
    }

    fn attempt(&mut self, step: &StepAttempt) {
        self.set("step", step.step);
        self.set("attempt", step.attempt);
    }

    fn hook(&mut self, hook: &HookSource) {
        match hook {
            HookSource::Step {
                policy, hook, step, ..
            } => {
                self.attempt(step);
                self.set("policy", policy);
                self.set("hook", hook);
            }
            HookSource::Workflow { policy, hook, .. } => {
                self.set("policy", policy);
                self.set("hook", hook);
            }
            _ => {}
        }
    }

    fn requester(&mut self, requester: &Requester) {
        match requester {
            Requester::Step { step, .. } => self.set("step", step),
            Requester::Adapter { adapter, step, .. } => {
                self.set("adapter", adapter);
                self.set("step", step);
            }
            Requester::StepHook {
                policy, hook, step, ..
            } => {
                self.set("policy", policy);
                self.set("hook", hook);
                self.set("step", step);
            }
            Requester::WorkflowHook { policy, hook, .. } => {
                self.set("policy", policy);
                self.set("hook", hook);
            }
            _ => {}
        }
    }

    fn source(&mut self, source: &Source) {
        match source {
            Source::Step(step) => {
                self.attempt(step);
                self.set("source", step.step);
            }
            Source::Hook(hook) => {
                self.hook(hook);
                if let (Some(policy), Some(hook)) =
                    (self.cells.get("policy"), self.cells.get("hook"))
                {
                    self.set("source", format!("{policy}, {hook}"));
                }
            }
            _ => {}
        }
    }

    fn reason(&mut self, reason: &Reason) {
        self.set("code", reason.code());
        if let Some(message) = reason.message() {
            self.carried.push(Value::from(message));
        }
        if let Some(details) = reason.details() {
            self.carried
                .push(serde_json::to_value(details).unwrap_or(Value::Null));
        }
    }

    /// Whether the event carries this message, as an emitted event's message, an error's
    /// message or a reason's message.
    pub(crate) fn carries_message(&self, message: &str) -> bool {
        ["message", "error"]
            .iter()
            .any(|column| self.has_cell(column, message))
            || self.carried.iter().any(|value| is_text(value, message))
    }

    /// Whether the event carries any of these values.
    pub(crate) fn carries_any(&self, values: &[&Value]) -> bool {
        values.iter().any(|value| self.carries_value(value))
    }

    /// Whether the event carries this value: within the values it carries, or written within
    /// the text it carries. The names it gives, such as a step's or a key's, are not data.
    pub(crate) fn carries_value(&self, value: &Value) -> bool {
        let text = match value {
            Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        let mut values = self.data.iter().chain(&self.carried);
        let mut texts = ["message", "error"]
            .iter()
            .filter_map(|column| self.cells.get(column));
        values.clone().any(|carried| holds(carried, value))
            || !text.is_empty()
                && (texts.any(|cell| cell.contains(&text))
                    || values.any(|carried| holds_text(carried, &text)))
    }

    fn emitted(&mut self, message: &str, data: Option<&itinera::value::AnyValue>) {
        self.set("message", message);
        self.data = data.and_then(as_json);
    }
}

impl From<&Event> for Line {
    fn from(event: &Event) -> Self {
        let mut line = Self {
            event: event.kind().to_owned(),
            ..Self::default()
        };
        match &event.body {
            EventBody::AttemptStarted { step, .. }
            | EventBody::StepSucceeded { step, .. }
            | EventBody::ContributionsDiscarded { step, .. } => line.attempt(step),
            EventBody::StepAbnormalTermination { step, message, .. } => {
                line.attempt(step);
                line.carried.push(Value::from(message.as_str()));
            }
            EventBody::InputAdapterSupplied {
                step, key, adapter, ..
            }
            | EventBody::InputAdapterFailed {
                step, key, adapter, ..
            } => {
                line.attempt(step);
                line.set("key", key);
                line.set("adapter", adapter);
            }
            EventBody::OptionalInputAbsent { key, requester, .. } => {
                line.set("key", key);
                match requester {
                    RequestSource::Step(step) => line.attempt(step),
                    RequestSource::Hook(hook) => line.hook(hook),
                    RequestSource::Adapter { adapter, step, .. } => {
                        line.attempt(step);
                        line.set("adapter", adapter);
                    }
                    _ => {}
                }
            }
            EventBody::StepFailed {
                step,
                retriable,
                reason,
                ..
            } => {
                line.attempt(step);
                line.set("retriable", retriable);
                line.reason(reason);
            }
            EventBody::StepSkipped { step, reason, .. } => {
                line.attempt(step);
                if let Some(reason) = reason {
                    line.reason(reason);
                }
            }
            EventBody::HookCalled {
                hook, lifecycle, ..
            } => {
                line.hook(hook);
                match lifecycle {
                    Some(lifecycle) => line.set("lifecycle", lifecycle),
                    None => line.set("lifecycle", "none"),
                }
            }
            EventBody::ContributionCommitted { key, source, .. }
            | EventBody::DataOverwritten { key, source, .. } => {
                line.set("key", key);
                line.source(source);
            }
            EventBody::JourneyAborted { abort, .. } => line.abort(abort),
            EventBody::StepRetrying { step, cause, .. } => {
                line.attempt(step);
                line.set("cause", cause);
                line.set("decided by", "default");
            }
            EventBody::StepGivenUp { step, cause, .. } => {
                line.attempt(step);
                line.set("cause", cause);
                match cause {
                    GiveUpCause::FailWorkflow {
                        decided_by, reason, ..
                    } => {
                        line.set(
                            "decided by",
                            format!("{}, {}", decided_by.policy, decided_by.hook),
                        );
                        line.reason(reason);
                    }
                    _ => line.set("decided by", "default"),
                }
            }
            EventBody::JourneySucceeded { decided_by, .. } => match decided_by {
                Some(policy) => line.set("decided by", format!("{policy}, on step success")),
                None => line.set("decided by", "default"),
            },
            EventBody::JourneyFailed { step, failure, .. } => {
                line.set("step", step);
                line.failure(failure);
            }
            EventBody::StepInfo {
                step,
                message,
                data,
                ..
            }
            | EventBody::StepWarning {
                step,
                message,
                data,
                ..
            }
            | EventBody::StepError {
                step,
                message,
                data,
                ..
            } => {
                line.attempt(step);
                line.emitted(message, data.as_ref());
            }
            EventBody::JourneyInfo {
                hook,
                message,
                data,
                ..
            }
            | EventBody::JourneyWarning {
                hook,
                message,
                data,
                ..
            }
            | EventBody::JourneyError {
                hook,
                message,
                data,
                ..
            } => {
                line.hook(hook);
                line.emitted(message, data.as_ref());
            }
            _ => {}
        }
        line
    }
}

impl Line {
    fn abort(&mut self, abort: &JourneyAbort) {
        self.set("code", abort.reason());
        if let Some(step) = abort.step() {
            self.set("step", step);
        }
        if let Some(error) = abort.error() {
            self.set("error", error);
        }
        match abort {
            JourneyAbort::PolicyCouldNotBeBuilt { policy, .. } => self.set("policy", policy),
            JourneyAbort::RequiredDataMissing { missing, .. } => match missing {
                MissingData::Key { key, requester, .. } => {
                    self.set("key", key);
                    self.requester(requester);
                }
                MissingData::Reason {
                    policy, hook, step, ..
                }
                | MissingData::Error {
                    policy, hook, step, ..
                } => {
                    self.set("policy", policy);
                    self.set("hook", hook);
                    self.set("step", step);
                }
                _ => {}
            },
            JourneyAbort::WrongType { key, requester, .. } => {
                self.set("key", key);
                self.requester(requester);
            }
            _ => {}
        }
    }

    fn failure(&mut self, failure: &JourneyFailure) {
        self.set("cause", failure.cause());
        self.set("decided by", "default");
        match failure {
            JourneyFailure::Failure(reason) => self.reason(reason),
            JourneyFailure::RetriesExhausted(LastFailure::Reason(reason)) => self.reason(reason),
            JourneyFailure::RetriesExhausted(LastFailure::Error(error))
            | JourneyFailure::AbnormalTermination(error) => self.set("error", error),
            JourneyFailure::FailWorkflow {
                decided_by, reason, ..
            } => {
                self.set(
                    "decided by",
                    format!("{}, {}", decided_by.policy, decided_by.hook),
                );
                self.reason(reason);
            }
            _ => {}
        }
    }
}

/// Whether an event table may not have this column.
fn is_unknown_column(column: &&str) -> bool {
    *column != "event" && !COLUMNS.contains(column)
}

fn holds(carried: &Value, value: &Value) -> bool {
    carried == value
        || match carried {
            Value::Array(items) => items.iter().any(|item| holds(item, value)),
            Value::Object(entries) => entries.values().any(|item| holds(item, value)),
            _ => false,
        }
}

/// Whether the value is this text.
fn is_text(value: &Value, text: &str) -> bool {
    value.as_str() == Some(text)
}

/// Event data as JSON, to compare with what a table writes.
fn as_json(data: &itinera::value::AnyValue) -> Option<Value> {
    serde_json::to_value(data).ok()
}

fn holds_text(carried: &Value, text: &str) -> bool {
    match carried {
        Value::String(carried) => carried.contains(text),
        Value::Array(items) => items.iter().any(|item| holds_text(item, text)),
        Value::Object(entries) => {
            entries.keys().any(|key| key.contains(text))
                || entries.values().any(|item| holds_text(item, text))
        }
        _ => false,
    }
}

/// Whether the events include these lines in this order, possibly with others in between.
pub(crate) fn includes_in_order(events: &[Line], expected: &[Line]) -> bool {
    let mut events = events.iter();
    expected.iter().all(|line| follows(&mut events, line))
}

/// Whether a later event matches the line, consuming the events up to it.
fn follows<'a>(events: &mut impl Iterator<Item = &'a Line>, line: &Line) -> bool {
    events.any(|event| line.matches(event))
}

/// Whether the events are exactly these lines.
pub(crate) fn are_exactly(events: &[Line], expected: &[Line]) -> bool {
    events.len() == expected.len()
        && expected
            .iter()
            .zip(events)
            .all(|(line, event)| line.matches(event))
}

#[cfg(test)]
mod tests {
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
}
