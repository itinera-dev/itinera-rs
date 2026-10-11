//! What the scripts of a policy's hooks say, read from the scenario model and typed for each
//! hook.

use itinera::policy::{
    HookNeeds, StepAbnormalTermination, StepFailure, StepHook, StepRetry, StepSuccess,
    WorkflowFailure, WorkflowHook, WorkflowSuccess,
};
use serde_json::Value;

use super::request::Request;
use super::returning::Returning;
use super::scripting::Scripting;
use crate::model::{self, HookAction, HookScript, Hooks, Level, ModelError};
use crate::value::Typed;

/// What the scripts of a policy's hooks say, each typed for its hook, and the name of the
/// policy; a hook it does not define has no script.
#[derive(Debug, Default)]
pub(crate) struct Scripts {
    policy: String,
    pub(super) on_step_success: Option<Script<StepSuccess>>,
    pub(super) on_step_failure: Option<Script<StepFailure>>,
    pub(super) on_step_retry: Option<Script<StepRetry>>,
    pub(super) on_step_abnormal_termination: Option<Script<StepAbnormalTermination>>,
    pub(super) on_workflow_success: Option<Script<WorkflowSuccess>>,
    pub(super) on_workflow_failure: Option<Script<WorkflowFailure>>,
}

impl Scripts {
    /// The scripts of the policy's hooks, typed.
    pub(crate) fn of(name: &str, policy: &model::Policy) -> Result<Self, ModelError> {
        let none = Self {
            policy: name.to_owned(),
            ..Self::default()
        };
        match &policy.hooks {
            Hooks::Step(hooks) => hooks.iter().try_fold(none, Self::with_step_hook),
            Hooks::Workflow(hooks) => hooks.iter().try_fold(none, Self::with_workflow_hook),
        }
    }

    fn with_step_hook(self, (hook, script): &(StepHook, HookScript)) -> Result<Self, ModelError> {
        Ok(match hook {
            StepHook::OnStepSuccess => Self {
                on_step_success: Some(Script::of(script)?),
                ..self
            },
            StepHook::OnStepFailure => Self {
                on_step_failure: Some(Script::of(script)?),
                ..self
            },
            StepHook::OnStepRetry => Self {
                on_step_retry: Some(Script::of(script)?),
                ..self
            },
            StepHook::OnStepAbnormalTermination => Self {
                on_step_abnormal_termination: Some(Script::of(script)?),
                ..self
            },
            _ => return Err(ModelError::UnknownHook(hook.to_string())),
        })
    }

    fn with_workflow_hook(
        self,
        (hook, script): &(WorkflowHook, HookScript),
    ) -> Result<Self, ModelError> {
        Ok(match hook {
            WorkflowHook::OnWorkflowSuccess => Self {
                on_workflow_success: Some(Script::of(script)?),
                ..self
            },
            WorkflowHook::OnWorkflowFailure => Self {
                on_workflow_failure: Some(Script::of(script)?),
                ..self
            },
            _ => return Err(ModelError::UnknownHook(hook.to_string())),
        })
    }

    /// What the policy's hook of kind `H` needs.
    pub(crate) fn needs<H: Scripting>(&self) -> Result<HookNeeds<H>, ModelError> {
        H::script(self)
            .map(Script::needs)
            .ok_or_else(|| ModelError::HookNotDefined(self.policy.clone(), H::HOOK))
    }
}

/// What one hook of kind `H` requests, does and returns.
#[derive(Debug)]
pub(crate) struct Script<H: Scripting> {
    needs: HookNeeds<H>,
    pub(super) requests: Vec<Request>,
    pub(super) actions: Vec<Action>,
    pub(super) returns: Returning<H::Returns>,
}

impl<H: Scripting> Script<H> {
    fn of(script: &HookScript) -> Result<Self, ModelError> {
        let requests: Vec<Request> = script.requests.iter().filter_map(Request::of).collect();
        let needs = requests
            .iter()
            .try_fold(HookNeeds::new().contributor().reporter(), H::declare)
            .map_err(not_allowed::<H>)?;
        Ok(Self {
            needs,
            requests,
            actions: script
                .actions
                .iter()
                .filter_map(Action::of)
                .collect::<Result<_, _>>()?,
            returns: H::returning(&script.returns)?,
        })
    }

    fn needs(&self) -> HookNeeds<H> {
        self.needs.clone()
    }
}

/// A request a hook of kind `H` cannot make.
fn not_allowed<H: Scripting>(request: String) -> ModelError {
    ModelError::RequestNotAllowed(H::HOOK, request)
}

/// What a hook does before it returns, with its values typed.
///
/// Rust gives a hook its own copy of what it receives, so a hook that changes it changes
/// nothing anyone else sees. The script leaves those changes out.
#[derive(Debug)]
pub(super) enum Action {
    Contribute(String, Typed),
    /// Adds 1 to the policy instance's counter, and contributes the count under this key.
    CountCalls(String),
    Emit(Level, String),
    CallRole {
        role: String,
        operation: String,
    },
}

impl Action {
    fn of(action: &HookAction) -> Option<Result<Self, ModelError>> {
        match action {
            HookAction::ChangeStepData { .. } => None,
            HookAction::Contribute { key, value } => Some(contribution(key, value)),
            HookAction::ContributeCallCount { key } => Some(Ok(Self::CountCalls(key.clone()))),
            HookAction::Emit { level, message } => Some(Ok(Self::Emit(*level, message.clone()))),
            HookAction::CallRole { role, operation } => Some(Ok(Self::CallRole {
                role: role.clone(),
                operation: operation.clone(),
            })),
        }
    }
}

fn contribution(key: &str, value: &Value) -> Result<Action, ModelError> {
    Ok(Action::Contribute(key.to_owned(), Typed::of(value)?))
}
