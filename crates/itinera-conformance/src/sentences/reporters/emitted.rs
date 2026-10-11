//! Events and reporters: the events steps and hooks emit.

use cucumber::given;

use crate::model::{HookAction, Level, ModelError, StepAction, json};
use crate::world::World;

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
