//! What an executor prepares before a journey starts: the workflow's policies, built for it, and
//! the initial global metadata, with the instance whose journey is ready to start.

use super::Refusal;
use crate::engine::{Failures, WorkflowPolicies};
use crate::instance::WorkflowInstance;
use crate::policy::{BuiltWorkflowPolicy, WorkflowPolicyEntry};
use crate::report::GlobalMetadata;

/// An instance whose journey is ready to start: the instance, its workflow policies, the
/// delivery of its events and the failures of its reporters.
pub(crate) struct Started<I: WorkflowInstance, D> {
    pub(crate) instance: I,
    pub(crate) policies: WorkflowPolicies<I::Workflow, I::Mode>,
    pub(crate) delivery: D,
    pub(crate) failures: Failures,
}

/// Takes the instance's initial global metadata, unless it sets the journey ID's own key.
pub(super) fn initial_global_metadata<I: WorkflowInstance>(
    instance: &mut I,
) -> Result<GlobalMetadata, Refusal> {
    let metadata = instance.take_global_metadata();
    if metadata.sets_journey_id() {
        return Err(Refusal::ReservedMetadataKey);
    }
    Ok(metadata)
}

/// Builds one instance of each of the instance's workflow policies, for its journey, in the order
/// they were attached.
pub(crate) fn build_policies<I: WorkflowInstance>(
    instance: &I,
) -> Result<WorkflowPolicies<I::Workflow, I::Mode>, Refusal> {
    instance
        .descriptor()
        .policies()
        .iter()
        .map(Box::as_ref)
        .map(build_policy)
        .collect()
}

fn build_policy<W, M>(
    policy: &dyn WorkflowPolicyEntry<W, M>,
) -> Result<Box<dyn BuiltWorkflowPolicy<W, M>>, Refusal> {
    match policy.build() {
        Ok(built) => Ok(built),
        Err(error) => Err(Refusal::WorkflowPolicy {
            policy: policy.name(),
            error,
        }),
    }
}
