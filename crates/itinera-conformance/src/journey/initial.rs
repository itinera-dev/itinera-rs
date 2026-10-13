//! What a workflow instance is created with: the scenario's initial data and initial global
//! metadata, given to its builder as typed values.

use itinera::instance::InstanceBuilder;
use itinera::mode::Mode;
use itinera::value::Value as Storable;
use serde_json::Value;

use crate::declaration::ScriptedWorkflow;
use crate::model::ModelError;
use crate::value::{Takes, Typed};

/// The instance builder, with this entry of the initial data.
pub(super) fn with_data<M: Mode>(
    builder: InstanceBuilder<ScriptedWorkflow, M>,
    entry: &(String, Value),
) -> Result<InstanceBuilder<ScriptedWorkflow, M>, ModelError> {
    entering(builder, entry, Initial::Data)
}

/// The instance builder, with this entry of the initial global metadata.
pub(super) fn with_global_metadata<M: Mode>(
    builder: InstanceBuilder<ScriptedWorkflow, M>,
    entry: &(String, Value),
) -> Result<InstanceBuilder<ScriptedWorkflow, M>, ModelError> {
    entering(builder, entry, Initial::GlobalMetadata)
}

fn entering<M: Mode>(
    builder: InstanceBuilder<ScriptedWorkflow, M>,
    (key, value): &(String, Value),
    into: Initial,
) -> Result<InstanceBuilder<ScriptedWorkflow, M>, ModelError> {
    Ok(Typed::of(value)?.given_to(Entering { builder, key, into }))
}

/// What an instance is created with: initial data, or initial global metadata.
#[derive(Clone, Copy)]
enum Initial {
    Data,
    GlobalMetadata,
}

/// Puts a value in the initial data or the initial global metadata.
struct Entering<'k, M: Mode> {
    builder: InstanceBuilder<ScriptedWorkflow, M>,
    key: &'k str,
    into: Initial,
}

impl<M: Mode> Takes for Entering<'_, M> {
    type Output = InstanceBuilder<ScriptedWorkflow, M>;

    fn take<T: Storable>(self, value: T) -> Self::Output {
        match self.into {
            Initial::Data => self.builder.data(self.key, value),
            Initial::GlobalMetadata => self.builder.reporters_global_metadata(self.key, value),
        }
    }
}
