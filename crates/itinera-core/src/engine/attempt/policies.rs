//! Building a step's policies for one of its attempts.

use crate::engine::end::End;
use crate::error::Error;
use crate::event::JourneyAbort;
use crate::journey::Abort;
use crate::policy::{BuiltStepPolicy, PolicyName, StepPolicyEntry};
use crate::step::{StepDescriptor, StepName};

/// Builds the step's policies for one attempt, in the order they were attached.
pub(super) fn build_policies<W, M>(
    step: &StepDescriptor<W, M>,
) -> Result<Vec<Box<dyn BuiltStepPolicy<W, M>>>, End> {
    let name = step.name();
    step.policies()
        .iter()
        .map(Box::as_ref)
        .map(|policy| build_policy(name, policy))
        .collect()
}

fn build_policy<W, M>(
    step: StepName,
    policy: &dyn StepPolicyEntry<W, M>,
) -> Result<Box<dyn BuiltStepPolicy<W, M>>, End> {
    match policy.build() {
        Ok(built) => Ok(built),
        Err(error) => Err(policy_could_not_be_built(step, policy.name(), error)),
    }
}

fn policy_could_not_be_built(step: StepName, policy: PolicyName, error: Error) -> End {
    let reported = JourneyAbort::PolicyCouldNotBeBuilt {
        step,
        policy,
        error: error.to_string(),
    };
    End::aborted(Abort::PolicyCouldNotBeBuilt { policy, error }, reported)
}
