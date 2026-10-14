//! A scripted hook run alone, outside any journey, as a test runs one: with the handles that the
//! recorders of itinera's test support lend.

use itinera::error::Error;
use itinera::journey::{Contributions, Contributor};
use itinera::policy::{HookEventReporter, StepHook, WorkflowHook};

use super::script::{Action, Script, Scripts};
use super::scripting::Scripting;
use super::{Emits, ScriptedPolicy};
use crate::executor::Executor;
use crate::model::Hook;
use crate::role::Roles;
use crate::witness::{Caller, Reported};

impl ScriptedPolicy {
    /// Does what the hook's script says, with a contributor the contributions recorder lends
    /// and a reporter the hook event reporter lends, in the mode of the executor. It carries on
    /// after any call to its reporter that is interrupted. A hook that requests anything is
    /// never run alone, since a test would have to give it what it requests.
    pub(crate) async fn run_alone(
        &self,
        hook: Hook,
        roles: &dyn Roles,
        contributions: &mut Contributions,
        events: &mut HookEventReporter,
        executor: Executor,
    ) -> Result<(), Error> {
        let actions = self.scripts.alone(hook)?;
        let mut contributor = contributions.contributor();
        match executor {
            Executor::Local => {
                let mut reporter = events.reporter();
                self.acting_alone(hook, actions, roles, &mut contributor, &mut reporter)
                    .await
            }
            Executor::AsyncLocal => {
                let mut reporter = events.async_reporter();
                self.acting_alone(hook, actions, roles, &mut contributor, &mut reporter)
                    .await
            }
        }
    }

    /// Performs each action, recording each call to the reporter with whether it was
    /// interrupted, and carrying on after it.
    async fn acting_alone<R: Emits>(
        &self,
        hook: Hook,
        actions: &[Action],
        roles: &dyn Roles,
        contributor: &mut Contributor<'_>,
        reporter: &mut R,
    ) -> Result<(), Error> {
        for action in actions {
            let done = self.perform(action, roles, contributor, reporter).await;
            let setting = match action.asks() {
                Asks::Nothing => {
                    done?;
                    continue;
                }
                Asks::Emit => None,
                Asks::Set(key) => Some(key.to_owned()),
            };
            self.witness.reported(Reported {
                caller: Caller::Hook {
                    policy: self.name.to_owned(),
                    hook,
                },
                setting,
                interrupted: done.is_err(),
            });
        }
        Ok(())
    }
}

impl Scripts {
    /// What the hook does, when it requests nothing.
    fn alone(&self, hook: Hook) -> Result<&[Action], Error> {
        match hook {
            Hook::Step(StepHook::OnStepSuccess) => actions(self.on_step_success.as_ref(), hook),
            Hook::Step(StepHook::OnStepFailure) => actions(self.on_step_failure.as_ref(), hook),
            Hook::Step(StepHook::OnStepRetry) => actions(self.on_step_retry.as_ref(), hook),
            Hook::Step(StepHook::OnStepAbnormalTermination) => {
                actions(self.on_step_abnormal_termination.as_ref(), hook)
            }
            Hook::Workflow(WorkflowHook::OnWorkflowSuccess) => {
                actions(self.on_workflow_success.as_ref(), hook)
            }
            Hook::Workflow(WorkflowHook::OnWorkflowFailure) => {
                actions(self.on_workflow_failure.as_ref(), hook)
            }
            Hook::Step(_) | Hook::Workflow(_) => {
                Err(Error::msg(format!("the hook {hook} has no script")))
            }
        }
    }
}

fn actions<H: Scripting>(script: Option<&Script<H>>, hook: Hook) -> Result<&[Action], Error> {
    match script {
        None => Err(Error::msg(format!("the hook {hook} has no script"))),
        Some(script) if !script.requests.is_empty() => Err(Error::msg(format!(
            "the hook {hook} requests what a hook run alone is not given"
        ))),
        Some(script) => Ok(&script.actions),
    }
}

/// What an action asks of the hook's reporter.
enum Asks<'a> {
    Nothing,
    Emit,
    /// To set the entry of global metadata under this key.
    Set(&'a str),
}

impl Action {
    fn asks(&self) -> Asks<'_> {
        match self {
            Self::Emit(..) => Asks::Emit,
            Self::SetGlobalMetadata(key, _) => Asks::Set(key),
            Self::Contribute(..) | Self::CountCalls(_) | Self::CallRole { .. } => Asks::Nothing,
        }
    }
}
