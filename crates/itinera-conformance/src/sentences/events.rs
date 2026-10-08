//! Events: comparing the stream with an event table.

use cucumber::gherkin::Step as Sentence;
use cucumber::then;

use super::{Unmet, expect, stream};
use crate::model::rows;
use crate::trace::{Line, are_exactly, includes_in_order};
use crate::world::World;

fn expected(sentence: &Sentence) -> Result<Vec<Line>, Unmet> {
    Ok(rows(sentence)?
        .iter()
        .map(Line::expected)
        .collect::<Result<_, _>>()?)
}

#[then(expr = "the events include, in order:")]
fn the_events_include_in_order(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let expected = expected(sentence)?;
    let (events, lines) = stream(world)?;
    expect(
        includes_in_order(&lines, &expected),
        "the events of the table in this order",
        &events,
    )
}

#[then(expr = "the events are exactly:")]
fn the_events_are_exactly(world: &mut World, #[step] sentence: &Sentence) -> Result<(), Unmet> {
    let expected = expected(sentence)?;
    let (events, lines) = stream(world)?;
    expect(
        are_exactly(&lines, &expected),
        "exactly the events of the table",
        &events,
    )
}
