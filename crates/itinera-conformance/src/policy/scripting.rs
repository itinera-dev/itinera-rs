//! The kinds of hook, as the scripts declare what each requests, read what it received and say
//! what it returns.

use itinera::error::Error;
use itinera::policy::{
    FailWorkflow, HookNeeds, OnSuccess, PolicyHookKind, Requested, StepAbnormalTermination,
    StepFailure, StepHook, StepRetry, StepSuccess, WorkflowFailure, WorkflowHook, WorkflowSuccess,
};
use itinera::step::Reason;

use super::needs::{declare_for_any, declare_for_error, declare_for_failure, declare_for_step};
use super::request::Request;
use super::requested::{receive_for_any, receive_for_error, receive_for_failure, receive_for_step};
use super::returning::{Returning, failing_returning, fails, workflow_returning};
use super::script::{Script, Scripts};
use crate::declaration::ScriptedWorkflow;
use crate::model::{Hook, HookReturn, ModelError};
use crate::witness::Received;

/// A kind of hook, as the scripts declare what it requests, read what it received and say what
/// it returns.
pub(crate) trait Scripting: PolicyHookKind + Sized {
    const HOOK: Hook;

    /// What a hook of this kind returns when it does not fail.
    type Returns: Clone + std::fmt::Debug + Send + Sync;

    fn script(scripts: &Scripts) -> Option<&Script<Self>>;

    /// What the scenario says the hook returns, if a hook of this kind may return it.
    fn returning(returned: &HookReturn) -> Result<Returning<Self::Returns>, ModelError>;

    /// Declares one request, or names it when this kind may not make it.
    fn declare(needs: HookNeeds<Self>, request: &Request) -> Result<HookNeeds<Self>, String>;

    /// Takes what the hook received for one request.
    fn receive<M>(
        got: &mut Requested<'_, ScriptedWorkflow, Self, M>,
        request: &Request,
        received: &mut Received,
    ) -> Result<(), Error>;
}

impl Scripting for StepSuccess {
    const HOOK: Hook = Hook::Step(StepHook::OnStepSuccess);

    type Returns = Option<OnSuccess>;

    fn script(scripts: &Scripts) -> Option<&Script<Self>> {
        scripts.on_step_success.as_ref()
    }

    fn returning(returned: &HookReturn) -> Result<Returning<Self::Returns>, ModelError> {
        Ok(match returned {
            HookReturn::Nothing => Ok(None),
            HookReturn::FinishWorkflow => Ok(Some(OnSuccess::FinishWorkflow)),
            HookReturn::FailWorkflow { code } => {
                Ok(Some(OnSuccess::FailWorkflow(Reason::new(code.clone()))))
            }
            HookReturn::Fails(message) => fails(message.as_deref()),
        })
    }

    fn declare(needs: HookNeeds<Self>, request: &Request) -> Result<HookNeeds<Self>, String> {
        declare_for_step(needs, request)
    }

    fn receive<M>(
        got: &mut Requested<'_, ScriptedWorkflow, Self, M>,
        request: &Request,
        received: &mut Received,
    ) -> Result<(), Error> {
        receive_for_step(got, request, received)
    }
}

impl Scripting for StepFailure {
    const HOOK: Hook = Hook::Step(StepHook::OnStepFailure);

    type Returns = Option<FailWorkflow>;

    fn script(scripts: &Scripts) -> Option<&Script<Self>> {
        scripts.on_step_failure.as_ref()
    }

    fn returning(returned: &HookReturn) -> Result<Returning<Self::Returns>, ModelError> {
        failing_returning(Self::HOOK, returned)
    }

    fn declare(needs: HookNeeds<Self>, request: &Request) -> Result<HookNeeds<Self>, String> {
        match request {
            Request::FailureCause => Ok(needs),
            _ => declare_for_failure(needs, request),
        }
    }

    fn receive<M>(
        got: &mut Requested<'_, ScriptedWorkflow, Self, M>,
        request: &Request,
        received: &mut Received,
    ) -> Result<(), Error> {
        match request {
            Request::FailureCause => received.cause = Some(got.cause().to_string()),
            _ => return receive_for_failure(got, request, received),
        }
        Ok(())
    }
}

impl Scripting for StepRetry {
    const HOOK: Hook = Hook::Step(StepHook::OnStepRetry);

    type Returns = Option<FailWorkflow>;

    fn script(scripts: &Scripts) -> Option<&Script<Self>> {
        scripts.on_step_retry.as_ref()
    }

    fn returning(returned: &HookReturn) -> Result<Returning<Self::Returns>, ModelError> {
        failing_returning(Self::HOOK, returned)
    }

    fn declare(needs: HookNeeds<Self>, request: &Request) -> Result<HookNeeds<Self>, String> {
        match request {
            Request::RetryCause => Ok(needs),
            _ => declare_for_failure(needs, request),
        }
    }

    fn receive<M>(
        got: &mut Requested<'_, ScriptedWorkflow, Self, M>,
        request: &Request,
        received: &mut Received,
    ) -> Result<(), Error> {
        match request {
            Request::RetryCause => received.cause = Some(got.cause().to_string()),
            _ => return receive_for_failure(got, request, received),
        }
        Ok(())
    }
}

impl Scripting for StepAbnormalTermination {
    const HOOK: Hook = Hook::Step(StepHook::OnStepAbnormalTermination);

    type Returns = Option<FailWorkflow>;

    fn script(scripts: &Scripts) -> Option<&Script<Self>> {
        scripts.on_step_abnormal_termination.as_ref()
    }

    fn returning(returned: &HookReturn) -> Result<Returning<Self::Returns>, ModelError> {
        failing_returning(Self::HOOK, returned)
    }

    fn declare(needs: HookNeeds<Self>, request: &Request) -> Result<HookNeeds<Self>, String> {
        declare_for_error(needs, request)
    }

    fn receive<M>(
        got: &mut Requested<'_, ScriptedWorkflow, Self, M>,
        request: &Request,
        received: &mut Received,
    ) -> Result<(), Error> {
        receive_for_error(got, request, received)
    }
}

impl Scripting for WorkflowSuccess {
    const HOOK: Hook = Hook::Workflow(WorkflowHook::OnWorkflowSuccess);

    type Returns = ();

    fn script(scripts: &Scripts) -> Option<&Script<Self>> {
        scripts.on_workflow_success.as_ref()
    }

    fn returning(returned: &HookReturn) -> Result<Returning<()>, ModelError> {
        workflow_returning(Self::HOOK, returned)
    }

    fn declare(needs: HookNeeds<Self>, request: &Request) -> Result<HookNeeds<Self>, String> {
        declare_for_any(needs, request)
    }

    fn receive<M>(
        got: &mut Requested<'_, ScriptedWorkflow, Self, M>,
        request: &Request,
        received: &mut Received,
    ) -> Result<(), Error> {
        receive_for_any(got, request, received)
    }
}

impl Scripting for WorkflowFailure {
    const HOOK: Hook = Hook::Workflow(WorkflowHook::OnWorkflowFailure);

    type Returns = ();

    fn script(scripts: &Scripts) -> Option<&Script<Self>> {
        scripts.on_workflow_failure.as_ref()
    }

    fn returning(returned: &HookReturn) -> Result<Returning<()>, ModelError> {
        workflow_returning(Self::HOOK, returned)
    }

    fn declare(needs: HookNeeds<Self>, request: &Request) -> Result<HookNeeds<Self>, String> {
        declare_for_any(needs, request)
    }

    fn receive<M>(
        got: &mut Requested<'_, ScriptedWorkflow, Self, M>,
        request: &Request,
        received: &mut Received,
    ) -> Result<(), Error> {
        receive_for_any(got, request, received)
    }
}
