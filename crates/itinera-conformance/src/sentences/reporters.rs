//! Events and reporters.

use std::num::NonZeroU64;

use cucumber::{given, then};
use itinera::event::Event;

use super::{Names, Unmet, expect, stream};
use crate::model::{
    Dispatching, EventKind, Holding, HookAction, Level, ModelError, ReporterFailure, StepAction,
    json,
};
use crate::trace::Line;
use crate::world::World;

#[given(expr = "the workflow lists the reporters {names}")]
fn the_workflow_lists_the_reporters(world: &mut World, reporters: Names) -> Result<(), ModelError> {
    world.model.workflow_mut()?.reporters.extend(reporters);
    Ok(())
}

#[given(expr = "the executor uses its default dispatcher")]
fn the_executor_uses_its_default_dispatcher(world: &mut World) -> Result<(), ModelError> {
    dispatches(world, Dispatching::Default)
}

#[given(expr = "the executor is given a dispatcher holding the reporter {string}")]
fn given_a_dispatcher_holding(world: &mut World, reporter: String) -> Result<(), ModelError> {
    holding(world, reporter, Holding::AddsReporters)
}

#[given(
    expr = "the executor is given a dispatcher holding the reporter {string} that ignores added reporters"
)]
fn given_a_dispatcher_that_ignores_added_reporters(
    world: &mut World,
    reporter: String,
) -> Result<(), ModelError> {
    holding(world, reporter, Holding::IgnoresAddedReporters)
}

#[given(
    expr = "the executor is given a dispatcher holding the reporter {string} that throws when a reporter is added"
)]
fn given_a_dispatcher_that_fails_when_adding(
    world: &mut World,
    reporter: String,
) -> Result<(), ModelError> {
    holding(world, reporter, Holding::FailsWhenAdding)
}

#[given(
    expr = "the executor is given a dispatcher holding the reporter {string} that throws when dispatching {string}"
)]
fn given_a_dispatcher_that_fails_dispatching(
    world: &mut World,
    reporter: String,
    event: String,
) -> Result<(), ModelError> {
    let behaviour = Holding::FailsDispatching(EventKind::named(&event)?);
    holding(world, reporter, behaviour)
}

#[given(expr = "the executor is given a dispatcher factory that fails")]
fn given_a_dispatcher_factory_that_fails(world: &mut World) -> Result<(), ModelError> {
    dispatches(world, Dispatching::FailingFactory)
}

fn holding(world: &mut World, reporter: String, behaviour: Holding) -> Result<(), ModelError> {
    dispatches(
        world,
        Dispatching::Holding {
            reporter,
            behaviour,
        },
    )
}

fn dispatches(world: &mut World, dispatching: Dispatching) -> Result<(), ModelError> {
    if world.model.dispatching != Dispatching::Unstated {
        return Err(ModelError::StatedTwice("the dispatcher"));
    }
    world.model.dispatching = dispatching;
    Ok(())
}

#[given(expr = "the reporter {string} throws")]
fn the_reporter_throws(world: &mut World, reporter: String) -> Result<(), ModelError> {
    fails(world, reporter, ReporterFailure::First)
}

#[given(expr = "the reporter {string} throws on {string}")]
fn the_reporter_throws_on(
    world: &mut World,
    reporter: String,
    event: String,
) -> Result<(), ModelError> {
    let failure = ReporterFailure::On(EventKind::named(&event)?, None);
    fails(world, reporter, failure)
}

#[given(expr = "the reporter {string} fails with {string} on {string}")]
fn the_reporter_fails_with_on(
    world: &mut World,
    reporter: String,
    message: String,
    event: String,
) -> Result<(), ModelError> {
    let failure = ReporterFailure::On(EventKind::named(&event)?, Some(message));
    fails(world, reporter, failure)
}

fn fails(world: &mut World, reporter: String, failure: ReporterFailure) -> Result<(), ModelError> {
    if world.model.reporter_failures.contains_key(&reporter) {
        return Err(ModelError::StatedTwice("how the reporter fails"));
    }
    world.model.reporter_failures.insert(reporter, failure);
    Ok(())
}

/// The "emits" sentences of a step with no data or with data written as JSON.
#[given(
    regex = r#"^step "([^"]*)" emits (\w+) "([^"]*)"(?: with data ([\[{"0-9-].*|true|false|null))?$"#
)]
fn step_emits(
    world: &mut World,
    step_name: String,
    kind: String,
    message: String,
    data: String,
) -> Result<(), ModelError> {
    let action = StepAction::Emit {
        level: Level::of("step", &kind)?,
        message,
        data: (!data.is_empty()).then(|| json(&data)).transpose()?,
    };
    world.model.step_mut(&step_name)?.actions.push(action);
    Ok(())
}

#[given(regex = r#"^the hook "([^"]*)" of policy "([^"]*)" emits (\w+) "([^"]*)"$"#)]
fn the_hook_emits(
    world: &mut World,
    hook: String,
    policy: String,
    kind: String,
    message: String,
) -> Result<(), ModelError> {
    let action = HookAction::Emit {
        level: Level::of("journey", &kind)?,
        message,
    };
    world.model.hook_mut(&policy, &hook)?.actions.push(action);
    Ok(())
}

#[then(expr = "the reporters {names} received the same events")]
fn the_reporters_received_the_same_events(
    world: &mut World,
    reporters: Names,
) -> Result<(), Unmet> {
    let mut received = reporters
        .as_ref()
        .iter()
        .map(|name| received_by(world, name));
    let Some(first) = received.next().transpose()? else {
        return Ok(());
    };
    received.try_for_each(|other| received_the_same(other, &first))
}

/// Holds when the other reporter's events could be read and are the same as the first's.
fn received_the_same(
    other: Result<Received<'_>, Unmet>,
    first: &Received<'_>,
) -> Result<(), Unmet> {
    let (name, events, lines) = other?;
    expect(
        lines == first.2,
        format_args!("\"{name}\" to receive the events \"{}\" received", first.0),
        &events,
    )
}

/// What the named reporter received: its events, and each one's sequence number and line.
fn received_by<'a>(world: &World, name: &'a str) -> Result<Received<'a>, Unmet> {
    let events = world.recorders.get(name)?.events();
    let lines = events.iter().map(numbered).collect();
    Ok((name, events, lines))
}

type Received<'a> = (&'a str, Vec<Event>, Vec<(NonZeroU64, Line)>);

/// The event's sequence number and its line, so that two reporters' streams compare whole.
fn numbered(event: &Event) -> (NonZeroU64, Line) {
    (event.sequence, Line::from(event))
}

#[then(expr = "the reporter {string} received the event {string}")]
fn the_reporter_received_the_event(
    world: &mut World,
    reporter: String,
    event: String,
) -> Result<(), Unmet> {
    received(world, &reporter, &event, Times::AtLeastOnce)
}

#[then(expr = "the reporter {string} received the event {string} {int} times")]
fn the_reporter_received_the_event_times(
    world: &mut World,
    reporter: String,
    event: String,
    times: usize,
) -> Result<(), Unmet> {
    received(world, &reporter, &event, Times::Exactly(times))
}

#[then(expr = "the reporter {string} did not receive the event {string}")]
fn the_reporter_did_not_receive_the_event(
    world: &mut World,
    reporter: String,
    event: String,
) -> Result<(), Unmet> {
    received(world, &reporter, &event, Times::Never)
}

fn received(world: &World, reporter: &str, event: &str, times: Times) -> Result<(), Unmet> {
    let kind = EventKind::named(event)?;
    let events = world.recorders.get(reporter)?.events();
    let count = events
        .iter()
        .filter(|received| kind.is_of(received))
        .count();
    expect(
        times.is_met_by(count),
        format_args!("\"{reporter}\" to receive {event} {times}"),
        &events,
    )
}

/// How often a reporter is expected to receive an event.
#[derive(Clone, Copy, Debug, derive_more::Display)]
enum Times {
    #[display("at least once")]
    AtLeastOnce,
    #[display("{_0} times")]
    Exactly(usize),
    #[display("never")]
    Never,
}

impl Times {
    fn is_met_by(self, count: usize) -> bool {
        match self {
            Self::AtLeastOnce => count > 0,
            Self::Exactly(times) => count == times,
            Self::Never => count == 0,
        }
    }
}

#[then(expr = "the reporter {string} received no event")]
fn the_reporter_received_no_event(world: &mut World, reporter: String) -> Result<(), Unmet> {
    let events = world.recorders.get(&reporter)?.events();
    expect(
        events.is_empty(),
        format_args!("\"{reporter}\" to receive no event"),
        &events,
    )
}

#[then(
    expr = "every event carries the journey ID {string} and the workflow name {string}, with increasing sequence numbers"
)]
fn every_event_carries_the_journey_id_and_workflow_name(
    world: &mut World,
    id: String,
    workflow: String,
) -> Result<(), Unmet> {
    let (events, _) = stream(world)?;
    let carried = events.iter().all(|event| belongs_to(event, &id, &workflow));
    let increasing = events.windows(2).all(in_sequence);
    expect(
        !events.is_empty() && carried && increasing,
        format_args!("every event of \"{workflow}\" for \"{id}\", in increasing sequence"),
        &events,
    )
}

/// Whether the event is of this journey of this workflow.
fn belongs_to(event: &Event, id: &str, workflow: &str) -> bool {
    event.journey_id.to_string() == id && event.workflow == workflow
}

/// Whether two consecutive events have increasing sequence numbers.
fn in_sequence(pair: &[Event]) -> bool {
    matches!(pair, [earlier, later] if earlier.sequence < later.sequence)
}

#[then(expr = "no engine event carries the value of {string}")]
fn no_engine_event_carries_the_value_of(world: &mut World, key: String) -> Result<(), Unmet> {
    let values = world.model.values_of(&key);
    if values.is_empty() {
        return Err(Unmet::Case(format!(
            "the scenario gives \"{key}\" no value"
        )));
    }
    let (events, lines) = stream(world)?;
    let carrying = events
        .iter()
        .zip(&lines)
        .filter(|(event, _)| is_engine_event(event))
        .any(|(_, line)| line.carries_any(&values));
    expect(
        !carrying,
        format_args!("no engine event to carry the value of \"{key}\""),
        &events,
    )
}

/// Whether the engine emitted the event, rather than a step or a hook emitting it.
fn is_engine_event(event: &Event) -> bool {
    !Level::is_emitted(event.kind())
}

#[then(expr = "no event carries the message {string}")]
fn no_event_carries_the_message(world: &mut World, message: String) -> Result<(), Unmet> {
    let (events, lines) = stream(world)?;
    let carrying = lines.iter().any(|line| line.carries_message(&message));
    expect(
        !carrying,
        format_args!("no event to carry \"{message}\""),
        &events,
    )
}
