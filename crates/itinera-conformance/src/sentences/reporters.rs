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
    let mut received = reporters.as_ref().iter().map(|name| received_by(world, name));
    let Some(first) = received.next().transpose()? else {
        return Ok(());
    };
    for other in received {
        let (name, events, lines) = other?;
        expect(
            lines == first.2,
            format_args!("\"{name}\" to receive the events \"{}\" received", first.0),
            &events,
        )?;
    }
    Ok(())
}

/// What the named reporter received: its events, and each one's sequence number and line.
fn received_by<'a>(world: &World, name: &'a str) -> Result<Received<'a>, Unmet> {
    let events = world.recorders.get(name)?.events();
    let lines = events
        .iter()
        .map(|event| (event.sequence, Line::from(event)))
        .collect();
    Ok((name, events, lines))
}

type Received<'a> = (&'a str, Vec<Event>, Vec<(NonZeroU64, Line)>);

#[then(expr = "the reporter {string} received the event {string}")]
fn the_reporter_received_the_event(
    world: &mut World,
    reporter: String,
    event: String,
) -> Result<(), Unmet> {
    received(world, &reporter, &event, |count| count > 0, "at least once")
}

#[then(expr = "the reporter {string} received the event {string} {int} times")]
fn the_reporter_received_the_event_times(
    world: &mut World,
    reporter: String,
    event: String,
    times: usize,
) -> Result<(), Unmet> {
    let how_often = format!("{times} times");
    received(world, &reporter, &event, |count| count == times, &how_often)
}

#[then(expr = "the reporter {string} did not receive the event {string}")]
fn the_reporter_did_not_receive_the_event(
    world: &mut World,
    reporter: String,
    event: String,
) -> Result<(), Unmet> {
    received(world, &reporter, &event, |count| count == 0, "never")
}

fn received(
    world: &World,
    reporter: &str,
    event: &str,
    count_holds: impl Fn(usize) -> bool,
    how_often: &str,
) -> Result<(), Unmet> {
    let kind = EventKind::named(event)?;
    let events = world.recorders.get(reporter)?.events();
    let count = events
        .iter()
        .filter(|received| received.kind() == kind.name())
        .count();
    expect(
        count_holds(count),
        format_args!("\"{reporter}\" to receive {event} {how_often}"),
        &events,
    )
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
    let carried = events
        .iter()
        .all(|event| event.journey_id.to_string() == id && event.workflow == workflow);
    let increasing = events
        .windows(2)
        .all(|pair| matches!(pair, [earlier, later] if earlier.sequence < later.sequence));
    expect(
        !events.is_empty() && carried && increasing,
        format_args!("every event of \"{workflow}\" for \"{id}\", in increasing sequence"),
        &events,
    )
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
        .filter(|(event, _)| !Level::is_emitted(event.kind()))
        .any(|(_, line)| values.iter().any(|value| line.carries_value(value)));
    expect(
        !carrying,
        format_args!("no engine event to carry the value of \"{key}\""),
        &events,
    )
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
