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
