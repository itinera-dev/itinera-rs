//! A step, a hook or an input adapter run alone, outside any journey, with handles lent by the
//! recorders of test support.

use cucumber::gherkin::Step as Sentence;
use cucumber::{given, when};
use itinera::journey::DataBag;
use itinera::policy::HookEventReporter;
use itinera::step::StepEventReporter;
use itinera::value::Value as Storable;
use serde_json::Value;

use super::entry;
use crate::declaration::answer_alone;
use crate::journey::Unrunnable;
use crate::model::{Hook, ModelError, rows};
use crate::policy::Building;
use crate::role::ProvidedRoles;
use crate::step::Scripted;
use crate::value::{Takes, Typed};
use crate::world::World;

#[given(expr = "the step event reporter is failing")]
fn the_step_event_reporter_is_failing(world: &mut World) {
    world.tested.step_events = StepEventReporter::failing();
}

#[given(expr = "the hook event reporter is failing")]
fn the_hook_event_reporter_is_failing(world: &mut World) {
    world.tested.hook_events = HookEventReporter::failing();
}

#[when(expr = "step {string} runs alone")]
async fn step_runs_alone(world: &mut World, step_name: String) -> Result<(), Unrunnable> {
    let executor = world.executor.ok_or(Unrunnable::NoExecutor)?;
    let script = world
        .model
        .workflow()?
        .scripts
        .get(&step_name)
        .ok_or_else(|| ModelError::UnknownStep(step_name.clone()))?;
    let step = Scripted::new(
        &step_name,
        script,
        world.steps_run.clone(),
        world.witness.clone(),
    )?;
    let tested = &mut world.tested;
    step.run_alone(&mut tested.contributions, &mut tested.step_events, executor)
        .await
        .map_err(Unrunnable::Alone)
}

#[when(expr = "the hook {string} of policy {string} runs alone")]
async fn the_hook_runs_alone(
    world: &mut World,
    hook: String,
    policy: String,
) -> Result<(), Unrunnable> {
    let executor = world.executor.ok_or(Unrunnable::NoExecutor)?;
    let declared = world
        .model
        .policies
        .get(&policy)
        .ok_or_else(|| ModelError::UnknownPolicy(policy.clone()))?;
    let scripted = Building::of(&policy, declared, &world.witness)?
        .build()
        .map_err(Unrunnable::Alone)?;
    let roles = ProvidedRoles::new(world.model.workflow()?.roles.clone(), world.witness.clone());
    let tested = &mut world.tested;
    scripted
        .run_alone(
            Hook::named(&hook)?,
            &roles,
            &mut tested.contributions,
            &mut tested.hook_events,
            executor,
        )
        .await
        .map_err(Unrunnable::Alone)
}

#[when(
    expr = "the input adapter {string} is called alone for the input {string} of step {string}, with a data bag holding:"
)]
fn the_input_adapter_is_called_alone(
    world: &mut World,
    adapter: String,
    input: String,
    step_name: String,
    #[step] sentence: &Sentence,
) -> Result<(), ModelError> {
    let bag = rows(sentence)?
        .iter()
        .map(entry)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .try_fold(DataBag::default(), with_entry)?;
    let declared = world
        .model
        .workflow()?
        .adapters
        .iter()
        .find(|declared| declared.is_named(&adapter))
        .ok_or_else(|| ModelError::UnknownAdapter(adapter.clone()))?;
    world.tested.adapter_returned = Some(answer_alone(declared, &input, &step_name, &bag)?);
    Ok(())
}

/// The data bag, holding this value under this key as well, as the Rust type of its type.
fn with_entry(bag: DataBag, (key, value): (String, Value)) -> Result<DataBag, ModelError> {
    Ok(Typed::of(&value)?.given_to(BagEntry { bag, key }))
}

/// Puts a value of whichever type it is in a data bag.
struct BagEntry {
    bag: DataBag,
    key: String,
}

impl Takes for BagEntry {
    type Output = DataBag;

    fn take<T: Storable>(self, value: T) -> DataBag {
        self.bag.with(self.key, value)
    }
}
