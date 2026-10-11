//! The instances of journey scope a journey's providers made, kept until the journey ends.

use std::sync::OnceLock;

use crate::mode::Mode;
use crate::service::{Instance, ProviderEntry, ProviderName, Scope};
use crate::workflow::WorkflowDescriptor;

/// The instances of journey scope a journey's providers made, kept until the journey ends: one
/// slot for each provider of journey scope, filled the first time a step it serves needs it.
#[derive(Debug)]
pub(in crate::engine) struct Kept {
    slots: Vec<(ProviderName, OnceLock<Instance>)>,
}

impl Kept {
    /// Empty slots for the workflow's providers of journey scope.
    pub(in crate::engine) fn of<W, M: Mode>(descriptor: &WorkflowDescriptor<W, M>) -> Self {
        Self {
            slots: descriptor
                .providers()
                .iter()
                .map(Box::as_ref)
                .filter(is_of_journey_scope)
                .map(empty_slot)
                .collect(),
        }
    }

    /// The slot of a provider of journey scope.
    pub(super) fn slot(&self, provider: ProviderName) -> Option<&OnceLock<Instance>> {
        self.slots
            .iter()
            .find(|slot| is_slot_of(slot, provider))
            .map(slot_of)
    }
}

fn is_of_journey_scope<W, M>(provider: &&dyn ProviderEntry<W, M>) -> bool {
    provider.scope() == Scope::Journey
}

fn empty_slot<W, M>(provider: &dyn ProviderEntry<W, M>) -> (ProviderName, OnceLock<Instance>) {
    (provider.name(), OnceLock::new())
}

fn is_slot_of((name, _): &&(ProviderName, OnceLock<Instance>), provider: ProviderName) -> bool {
    *name == provider
}

fn slot_of((_, slot): &(ProviderName, OnceLock<Instance>)) -> &OnceLock<Instance> {
    slot
}
