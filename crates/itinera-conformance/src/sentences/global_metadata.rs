//! Global metadata: what the workflow instance, steps, hooks and service providers set, and what
//! each reporter received.

mod received;

use cucumber::gherkin::Step as Sentence;
use cucumber::given;

use super::reporters::fails;
use crate::model::{
    HookAction, ModelError, ProviderAction, ReporterFailure, StepAction, json, rows,
};
use crate::world::World;

#[given(expr = "the workflow instance is created with the global metadata:")]
fn the_instance_is_created_with_the_global_metadata(
    world: &mut World,
    #[step] sentence: &Sentence,
) -> Result<(), ModelError> {
    for row in rows(sentence)? {
        let entry = (
            row.required("key")?.to_owned(),
            json(row.required("value")?)?,
        );
        world.model.initial_global_metadata.push(entry);
    }
    Ok(())
}

#[given(
    regex = r#"^step "([^"]*)" sets the global metadata "([^"]*)" to ([\[{"0-9-].*|true|false|null)$"#
)]
fn step_sets_the_global_metadata(
    world: &mut World,
    step_name: String,
    key: String,
    value: String,
) -> Result<(), ModelError> {
    let action = StepAction::SetGlobalMetadata {
        key,
        value: json(&value)?,
    };
    world.model.step_mut(&step_name)?.actions.push(action);
    Ok(())
}

#[given(
    regex = r#"^the hook "([^"]*)" of policy "([^"]*)" sets the global metadata "([^"]*)" to ([\[{"0-9-].*|true|false|null)$"#
)]
fn the_hook_sets_the_global_metadata(
    world: &mut World,
    hook: String,
    policy: String,
    key: String,
    value: String,
) -> Result<(), ModelError> {
    let action = HookAction::SetGlobalMetadata {
        key,
        value: json(&value)?,
    };
    world.model.hook_mut(&policy, &hook)?.actions.push(action);
    Ok(())
}

#[given(
    regex = r#"^the service provider "([^"]*)" sets the global metadata "([^"]*)" to ([\[{"0-9-].*|true|false|null)$"#
)]
fn the_service_provider_sets_the_global_metadata(
    world: &mut World,
    provider: String,
    key: String,
    value: String,
) -> Result<(), ModelError> {
    let action = ProviderAction::SetGlobalMetadata {
        key,
        value: json(&value)?,
    };
    world.model.provider_mut(&provider)?.actions.push(action);
    Ok(())
}

#[given(expr = "the reporter {string} fails when it receives the global metadata {string}")]
fn the_reporter_fails_on_the_global_metadata(
    world: &mut World,
    reporter: String,
    key: String,
) -> Result<(), ModelError> {
    fails(world, reporter, ReporterFailure::OnGlobalMetadata(key))
}
