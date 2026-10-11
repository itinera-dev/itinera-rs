//! How a hook ends: with what it returns, or failing with a message.

use itinera::policy::FailWorkflow;
use itinera::step::Reason;

use crate::model::{Hook, HookReturn, Lifecycle, ModelError};

/// How a hook ends: with what it returns, or failing with this message.
pub(crate) type Returning<R> = Result<R, String>;

/// A hook that fails, with the message the scenario gives, if any.
pub(super) fn fails<R>(message: Option<&str>) -> Returning<R> {
    Err(message.map_or_else(|| "the scripted hook fails".to_owned(), str::to_owned))
}

/// What a hook that may fail the workflow returns.
pub(super) fn failing_returning(
    hook: Hook,
    returned: &HookReturn,
) -> Result<Returning<Option<FailWorkflow>>, ModelError> {
    Ok(match returned {
        HookReturn::Nothing => Ok(None),
        HookReturn::FailWorkflow { code } => {
            Ok(Some(FailWorkflow::from(Reason::new(code.clone()))))
        }
        HookReturn::FinishWorkflow => {
            return Err(ModelError::LifecycleNotAllowed(
                hook,
                Lifecycle::FinishWorkflow,
            ));
        }
        HookReturn::Fails(message) => fails(message.as_deref()),
    })
}

/// What a workflow hook returns, which is never a lifecycle.
pub(super) fn workflow_returning(
    hook: Hook,
    returned: &HookReturn,
) -> Result<Returning<()>, ModelError> {
    match returned {
        HookReturn::Nothing => Ok(Ok(())),
        HookReturn::FinishWorkflow => Err(ModelError::LifecycleNotAllowed(
            hook,
            Lifecycle::FinishWorkflow,
        )),
        HookReturn::FailWorkflow { .. } => Err(ModelError::LifecycleNotAllowed(
            hook,
            Lifecycle::FailWorkflow,
        )),
        HookReturn::Fails(message) => Ok(fails(message.as_deref())),
    }
}

#[cfg(test)]
mod tests {
    use itinera::policy::{StepHook, WorkflowHook};
    use rstest::rstest;

    use super::*;
    use crate::model;
    use crate::policy::script::Scripts;

    fn returning(hook: Hook, returned: HookReturn) -> Result<Scripts, ModelError> {
        let mut policy = model::Policy::new(hook);
        policy.hook_mut(hook).unwrap().returns = returned;
        Scripts::of("policy", &policy)
    }

    #[rstest]
    #[case::finish_workflow_from_a_workflow_hook(
        Hook::Workflow(WorkflowHook::OnWorkflowSuccess),
        HookReturn::FinishWorkflow,
        Lifecycle::FinishWorkflow
    )]
    #[case::fail_workflow_from_a_workflow_hook(
        Hook::Workflow(WorkflowHook::OnWorkflowFailure),
        HookReturn::FailWorkflow { code: "late".to_owned() },
        Lifecycle::FailWorkflow
    )]
    #[case::finish_workflow_from_a_failure_hook(
        Hook::Step(StepHook::OnStepFailure),
        HookReturn::FinishWorkflow,
        Lifecycle::FinishWorkflow
    )]
    fn a_lifecycle_a_hook_cannot_return_is_a_case_error(
        #[case] hook: Hook,
        #[case] returned: HookReturn,
        #[case] lifecycle: Lifecycle,
    ) {
        assert_eq!(
            returning(hook, returned).unwrap_err(),
            ModelError::LifecycleNotAllowed(hook, lifecycle)
        );
    }
}
