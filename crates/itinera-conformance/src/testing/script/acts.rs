//! What the attempts a row of the script chooses do before they end, in itinera's own words.

use itinera::instance::{Acting, AttemptScripting, WorkflowInstance};
use itinera::value::Value as Storable;

use crate::model::{Acts, Level, ModelError};
use crate::testing::event_level;
use crate::value::{Takes, Typed};

/// Something a row's attempts do before they end.
pub(super) enum Act {
    Contribute(String, Typed),
    Emit(Level, String),
    SetGlobalMetadata(String, Typed),
}

/// What a row's attempts do, in order: its contributions, its event and its global metadata.
pub(super) fn actions(acts: &Acts) -> Result<Vec<Act>, ModelError> {
    let contributions = acts.contributes.iter().map(contribution);
    let emits = acts.emits.iter().map(emit).map(Ok);
    let sets = acts.sets.iter().map(global_metadata);
    contributions.chain(emits).chain(sets).collect()
}

fn contribution((key, value): &(String, serde_json::Value)) -> Result<Act, ModelError> {
    Ok(Act::Contribute(key.clone(), Typed::of(value)?))
}

fn emit((level, message): &(Level, String)) -> Act {
    Act::Emit(*level, message.clone())
}

fn global_metadata((key, value): &(String, serde_json::Value)) -> Result<Act, ModelError> {
    Ok(Act::SetGlobalMetadata(key.clone(), Typed::of(value)?))
}

pub(super) fn act<I: WorkflowInstance, S>(
    scripting: AttemptScripting<I, I::Mode, S>,
    action: Act,
) -> AttemptScripting<I, I::Mode, Acting> {
    match action {
        Act::Contribute(key, value) => value.given_to(Contributing { scripting, key }),
        Act::Emit(level, message) => scripting.will_emit(event_level(level), message),
        Act::SetGlobalMetadata(key, value) => value.given_to(Setting { scripting, key }),
    }
}

/// Scripts a contribution of a value of whichever type it is.
struct Contributing<I: WorkflowInstance, S> {
    scripting: AttemptScripting<I, I::Mode, S>,
    key: String,
}

impl<I: WorkflowInstance, S> Takes for Contributing<I, S> {
    type Output = AttemptScripting<I, I::Mode, Acting>;

    fn take<T: Storable>(self, value: T) -> Self::Output {
        self.scripting.will_contribute(self.key, value)
    }
}

/// Scripts an entry of global metadata of whichever type its value is.
struct Setting<I: WorkflowInstance, S> {
    scripting: AttemptScripting<I, I::Mode, S>,
    key: String,
}

impl<I: WorkflowInstance, S> Takes for Setting<I, S> {
    type Output = AttemptScripting<I, I::Mode, Acting>;

    fn take<T: Storable>(self, value: T) -> Self::Output {
        self.scripting.will_set_global_metadata(self.key, value)
    }
}
