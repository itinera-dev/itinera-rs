//! The tables that define policies, their hooks and what the hooks request, one row per hook
//! or request.

use cucumber::gherkin::Step as Sentence;
use cucumber::given;

use super::super::hooks::returns;
use super::requests;
use crate::model::{HookRequest, HookReturn, ModelError, Row, rows};
use crate::world::World;

const HOOK_COLUMNS: [&str; 5] = ["policy", "hook", "returns", "code", "fails with"];

const REQUEST_COLUMNS: [&str; 6] = ["policy", "hook", "request", "key", "type", "optional"];

#[given(expr = "the step policies and their hooks:")]
fn the_step_policies_and_their_hooks(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), ModelError> {
    rows(sentence)?
        .iter()
        .try_for_each(|row| define(world, row, true))
}

#[given(expr = "the workflow policies and their hooks:")]
fn the_workflow_policies_and_their_hooks(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), ModelError> {
    rows(sentence)?
        .iter()
        .try_for_each(|row| define(world, row, false))
}

/// Defines the row's hook, then gives it what its filled cells stand for.
fn define(world: &mut World, row: &Row, step_policy: bool) -> Result<(), ModelError> {
    row.only(&HOOK_COLUMNS)?;
    let policy = row.required("policy")?;
    let hook = row.required("hook")?;
    world.model.define(policy.to_owned(), hook, step_policy)?;
    let returned = match (row.optional("returns"), row.optional("code")) {
        (Some("FailWorkflow"), Some(code)) => Some(HookReturn::FailWorkflow {
            code: code.to_owned(),
        }),
        (Some("FailWorkflow"), None) => return Err(ModelError::MissingCell("code")),
        (_, Some(code)) => return Err(ModelError::Cell("code", code.to_owned())),
        (Some("FinishWorkflow"), None) => Some(HookReturn::FinishWorkflow),
        (Some(other), None) => return Err(ModelError::Cell("returns", other.to_owned())),
        (None, None) => None,
    };
    if let Some(returned) = returned {
        returns(world, policy, hook, returned)?;
    }
    match row.optional("fails with") {
        Some(message) => returns(world, policy, hook, fails(message)),
        None => Ok(()),
    }
}

fn fails(message: &str) -> HookReturn {
    HookReturn::Fails(Some(message.to_owned()))
}

#[given(expr = "the hooks request:")]
fn the_hooks_request(world: &mut World, #[step] sentence: &Sentence) -> Result<(), ModelError> {
    rows(sentence)?
        .iter()
        .try_for_each(|row| request(world, row))
}

fn request(world: &mut World, row: &Row) -> Result<(), ModelError> {
    row.only(&REQUEST_COLUMNS)?;
    let optional = match row.optional("optional") {
        Some("yes") => true,
        Some(other) => return Err(ModelError::Cell("optional", other.to_owned())),
        None => false,
    };
    let request = match (row.required("request")?, optional) {
        ("failure reason", optional) => HookRequest::FailureReason { optional },
        ("step data", optional) => HookRequest::StepData {
            key: row.required("key")?.to_owned(),
            value_type: row.parse("type")?,
            optional,
        },
        ("data from the workflow", optional) => HookRequest::DataFromWorkflow {
            key: row.required("key")?.to_owned(),
            value_type: row.parse("type")?,
            optional,
        },
        ("failure cause", false) => HookRequest::FailureCause,
        ("retry cause", false) => HookRequest::RetryCause,
        ("error", false) => HookRequest::Error,
        ("step name", false) => HookRequest::StepName,
        ("attempt number", false) => HookRequest::AttemptNumber,
        ("journey ID", false) => HookRequest::JourneyId,
        (other, _) => return Err(ModelError::Cell("request", other.to_owned())),
    };
    requests(
        world,
        row.required("policy")?,
        row.required("hook")?,
        request,
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::model::ValueType;

    fn audited() -> World {
        let mut world = World::default();
        world
            .model
            .define("audit".to_owned(), "on step failure", true)
            .unwrap();
        world
    }

    #[rstest]
    #[case::an_optional_failure_reason(
        &[("request", "failure reason"), ("optional", "yes")],
        HookRequest::FailureReason { optional: true },
    )]
    #[case::step_data(
        &[("request", "step data"), ("key", "amount"), ("type", "integer")],
        HookRequest::StepData {
            key: "amount".to_owned(),
            value_type: ValueType::Integer,
            optional: false,
        },
    )]
    #[case::the_journey_id(&[("request", "journey ID")], HookRequest::JourneyId)]
    fn a_row_declares_the_request_it_names(
        #[case] cells: &[(&str, &str)],
        #[case] declared: HookRequest,
    ) {
        let mut world = audited();
        let mut cells = cells.to_vec();
        cells.extend([("policy", "audit"), ("hook", "on step failure")]);

        request(&mut world, &Row::of(&cells)).unwrap();

        let script = world.model.hook_mut("audit", "on step failure").unwrap();
        assert_eq!(script.requests, [declared]);
    }

    #[test]
    fn a_request_with_no_optional_form_cannot_be_optional() {
        let mut world = audited();
        let cells = [
            ("policy", "audit"),
            ("hook", "on step failure"),
            ("request", "failure cause"),
            ("optional", "yes"),
        ];

        let declared = request(&mut world, &Row::of(&cells));

        assert_eq!(
            declared,
            Err(ModelError::Cell("request", "failure cause".to_owned()))
        );
    }

    #[test]
    fn a_row_defines_its_hook_with_what_it_returns() {
        let mut world = World::default();
        let cells = [
            ("policy", "guard"),
            ("hook", "on step failure"),
            ("returns", "FailWorkflow"),
            ("code", "blocked"),
        ];

        define(&mut world, &Row::of(&cells), true).unwrap();

        let script = world.model.hook_mut("guard", "on step failure").unwrap();
        assert_eq!(
            script.returns,
            HookReturn::FailWorkflow {
                code: "blocked".to_owned()
            }
        );
    }
}
