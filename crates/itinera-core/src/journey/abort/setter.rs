//! Who set global metadata under a key reserved to the executor, as the result tells it.

use crate::event::{HookSource, Source};
use crate::policy::{PolicyName, StepHook, WorkflowHook};
use crate::service::ProviderName;

/// Who set global metadata under a key reserved to the executor: a step, a step hook, a workflow
/// hook or a service provider.
///
/// It is the result's counterpart of [`event::Source`](crate::event::Source), without the step's
/// name or attempt, since the result names no step.
///
/// # Examples
///
/// ```
/// use itinera::journey::MetadataSetter;
///
/// fn by_a_step(setter: &MetadataSetter) -> bool {
///     matches!(setter, MetadataSetter::Step)
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MetadataSetter {
    /// A step.
    Step,
    /// A step hook.
    #[non_exhaustive]
    StepHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: StepHook,
    },
    /// A workflow hook.
    #[non_exhaustive]
    WorkflowHook {
        /// The policy's name.
        policy: PolicyName,
        /// The hook.
        hook: WorkflowHook,
    },
    /// A service provider.
    #[non_exhaustive]
    Provider {
        /// The provider's name.
        provider: ProviderName,
    },
}

impl From<&Source> for MetadataSetter {
    fn from(source: &Source) -> Self {
        match source {
            Source::Step(_) => Self::Step,
            Source::Hook(HookSource::Step { policy, hook, .. }) => Self::StepHook {
                policy: *policy,
                hook: *hook,
            },
            Source::Hook(HookSource::Workflow { policy, hook }) => Self::WorkflowHook {
                policy: *policy,
                hook: *hook,
            },
            Source::Hook(HookSource::Provider { provider, .. }) => Self::Provider {
                provider: *provider,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::step::{StepAttempt, StepName};

    fn lookup() -> StepAttempt {
        StepAttempt::first(StepName::new("lookup"))
    }

    #[rstest]
    #[case::a_step(Source::Step(lookup()), MetadataSetter::Step)]
    #[case::a_step_hook(
        Source::Hook(HookSource::Step {
            policy: PolicyName::from("tenant"),
            hook: StepHook::OnStepSuccess,
            step: lookup(),
        }),
        MetadataSetter::StepHook {
            policy: PolicyName::from("tenant"),
            hook: StepHook::OnStepSuccess,
        }
    )]
    #[case::a_workflow_hook(
        Source::Hook(HookSource::Workflow {
            policy: PolicyName::from("tenant"),
            hook: WorkflowHook::OnWorkflowSuccess,
        }),
        MetadataSetter::WorkflowHook {
            policy: PolicyName::from("tenant"),
            hook: WorkflowHook::OnWorkflowSuccess,
        }
    )]
    #[case::a_service_provider(
        Source::Hook(HookSource::Provider {
            provider: ProviderName::from("stock"),
            step: lookup(),
        }),
        MetadataSetter::Provider {
            provider: ProviderName::from("stock"),
        }
    )]
    fn the_setter_names_who_set_the_key_but_not_the_step(
        #[case] source: Source,
        #[case] setter: MetadataSetter,
    ) {
        assert_eq!(MetadataSetter::from(&source), setter);
    }
}
