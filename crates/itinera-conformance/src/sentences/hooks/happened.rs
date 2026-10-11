//! What happened to hooks: their calls, and what their lifecycles and contributions did to the
//! journey.

use cucumber::gherkin::Step as Sentence;
use cucumber::then;
use itinera::journey::{Failure, JourneyStatus};

use crate::model::{Hook, ModelError, Row, rows};
use crate::sentences::outcomes::fails_at;
use crate::sentences::received::calls_of;
use crate::sentences::{Unmet, expect, holds, result, stream};
use crate::trace::Line;
use crate::witness::Call;
use crate::world::World;

#[then(expr = "the hook {string} of policy {string} was called {int} times")]
fn the_hook_was_called_times(
    world: &mut World,
    hook: String,
    policy: String,
    times: usize,
) -> Result<(), Unmet> {
    called(world, &policy, &hook, times)
}

#[then(expr = "the hook {string} of policy {string} was not called")]
fn the_hook_was_not_called(world: &mut World, hook: String, policy: String) -> Result<(), Unmet> {
    called(world, &policy, &hook, 0)
}

fn called(world: &World, policy: &str, hook: &str, times: usize) -> Result<(), Unmet> {
    let found = calls_of(world, policy, hook)?.len();
    holds(
        found == times,
        format_args!("the hook \"{hook}\" of policy \"{policy}\" called {times} times"),
        format_args!("{found} calls"),
    )
}

#[then(expr = "the hooks were called in this order:")]
fn the_hooks_were_called_in_this_order(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), Unmet> {
    let expected = rows(sentence)?
        .iter()
        .map(policy_and_hook)
        .collect::<Result<Vec<_>, _>>()?;
    let found: Vec<(String, Hook)> = world.witness.calls().into_iter().map(named).collect();
    holds(
        found == expected,
        format_args!("the calls {expected:?}"),
        format_args!("{found:?}"),
    )
}

fn policy_and_hook(row: &Row) -> Result<(String, Hook), ModelError> {
    Ok((
        row.required("policy")?.to_owned(),
        Hook::named(row.required("hook")?)?,
    ))
}

fn named(call: Call) -> (String, Hook) {
    (call.policy, call.hook)
}

#[then(
    expr = "the result names the step {string} whose hook failed the journey, with the code {string}"
)]
fn the_result_names_the_step_whose_hook_failed_the_journey(
    world: &mut World,
    step_name: String,
    code: String,
) -> Result<(), Unmet> {
    let found = match &result(world)?.status {
        JourneyStatus::Failed {
            failure: Failure::FailWorkflow(reason),
            ..
        } => Some(reason.code()),
        _ => None,
    };
    let (events, lines) = stream(world)?;
    expect(
        found == Some(code.as_str()) && lines.iter().any(|line| fails_at(line, &step_name)),
        format_args!(
            "a FailWorkflow with the code \"{code}\", whose journey_failed names \"{step_name}\""
        ),
        &events,
    )
}

#[then(
    expr = "the contribution of {string} is recorded as made by the hook {string} of policy {string}"
)]
fn the_contribution_is_recorded_as_made_by_the_hook(
    world: &mut World,
    key: String,
    hook: String,
    policy: String,
) -> Result<(), Unmet> {
    let (events, lines) = stream(world)?;
    let source = format!("{policy}, {hook}");
    expect(
        lines.iter().any(|line| commits_from(line, &key, &source)),
        format_args!("a contribution_committed of \"{key}\" from {source}"),
        &events,
    )
}

fn commits_from(line: &Line, key: &str, source: &str) -> bool {
    line.event == "contribution_committed"
        && line.has_cell("key", key)
        && line.has_cell("source", source)
}
