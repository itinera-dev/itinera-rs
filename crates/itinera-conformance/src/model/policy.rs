//! What scripted policies and their hooks do, and the roles a workflow provides.

use std::collections::BTreeSet;

use itinera::policy::{StepHook, WorkflowHook};
use serde_json::Value;

use super::value::ValueType;
use super::{Level, ModelError};

/// A scripted policy: the hooks it defines, each with its script, in the order defined.
#[derive(Debug)]
pub(crate) struct Policy {
    pub(crate) hooks: Hooks,
    /// Whether building an instance of it fails, with this message when one is given.
    pub(crate) construction_failure: Option<Option<String>>,
}

/// The hooks of a policy, which are all of its own kind.
#[derive(Debug)]
pub(crate) enum Hooks {
    Step(Vec<(StepHook, HookScript)>),
    Workflow(Vec<(WorkflowHook, HookScript)>),
}

/// The name of a hook of either kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, derive_more::Display)]
pub(crate) enum Hook {
    Step(StepHook),
    Workflow(WorkflowHook),
}

impl Hook {
    const ALL: [Self; 6] = [
        Self::Step(StepHook::OnStepSuccess),
        Self::Step(StepHook::OnStepFailure),
        Self::Step(StepHook::OnStepRetry),
        Self::Step(StepHook::OnStepAbnormalTermination),
        Self::Workflow(WorkflowHook::OnWorkflowSuccess),
        Self::Workflow(WorkflowHook::OnWorkflowFailure),
    ];

    fn is_named(self, name: &str) -> bool {
        self.to_string() == name
    }

    pub(crate) fn named(name: &str) -> Result<Self, ModelError> {
        Self::ALL
            .into_iter()
            .find(|hook| hook.is_named(name))
            .ok_or_else(|| ModelError::UnknownHook(name.to_owned()))
    }
}

impl Policy {
    pub(crate) fn new(hook: Hook) -> Self {
        Self {
            hooks: match hook {
                Hook::Step(hook) => Hooks::Step(vec![(hook, HookScript::default())]),
                Hook::Workflow(hook) => Hooks::Workflow(vec![(hook, HookScript::default())]),
            },
            construction_failure: None,
        }
    }

    /// Adds a hook of the policy's own kind, which it does not define yet.
    pub(crate) fn define(&mut self, policy: &str, hook: Hook) -> Result<(), ModelError> {
        if self.hook_mut(hook).is_some() {
            return Err(ModelError::HookDefinedTwice(policy.to_owned(), hook));
        }
        match (&mut self.hooks, hook) {
            (Hooks::Step(hooks), Hook::Step(hook)) => hooks.push((hook, HookScript::default())),
            (Hooks::Workflow(hooks), Hook::Workflow(hook)) => {
                hooks.push((hook, HookScript::default()));
            }
            _ => return Err(ModelError::HookOfOtherKind(policy.to_owned(), hook)),
        }
        Ok(())
    }

    /// The scripts of its hooks, in the order defined.
    pub(crate) fn scripts(&self) -> Vec<&HookScript> {
        match &self.hooks {
            Hooks::Step(hooks) => hooks.iter().map(|(_, script)| script).collect(),
            Hooks::Workflow(hooks) => hooks.iter().map(|(_, script)| script).collect(),
        }
    }

    pub(crate) fn is_step_policy(&self) -> bool {
        matches!(self.hooks, Hooks::Step(_))
    }

    pub(crate) fn hook_mut(&mut self, hook: Hook) -> Option<&mut HookScript> {
        match (&mut self.hooks, hook) {
            (Hooks::Step(hooks), Hook::Step(hook)) => script_of(hooks, hook),
            (Hooks::Workflow(hooks), Hook::Workflow(hook)) => script_of(hooks, hook),
            _ => None,
        }
    }
}

/// The script of this hook, among hooks of one kind.
fn script_of<H: PartialEq>(hooks: &mut [(H, HookScript)], hook: H) -> Option<&mut HookScript> {
    hooks
        .iter_mut()
        .find(|entry| defines(entry, &hook))
        .map(|(_, script)| script)
}

/// Whether the entry is the script of this hook.
fn defines<H: PartialEq>((defined, _): &&mut (H, HookScript), hook: &H) -> bool {
    defined == hook
}

/// What a hook requests, what it does, and what it returns.
#[derive(Debug, Default)]
pub(crate) struct HookScript {
    /// Its parameters, in the order written.
    pub(crate) requests: Vec<HookRequest>,
    /// What it does before returning, in the order written.
    pub(crate) actions: Vec<HookAction>,
    pub(crate) returns: HookReturn,
}

#[derive(Debug, PartialEq)]
pub(crate) enum HookRequest {
    StepData {
        key: String,
        value_type: ValueType,
        optional: bool,
    },
    StepName,
    FailureReason {
        optional: bool,
    },
    FailureCause,
    RetryCause,
    Error,
    DataFromWorkflow {
        key: String,
        value_type: ValueType,
        optional: bool,
    },
    AttemptNumber,
    JourneyId,
    Role(String),
}

#[derive(Debug, PartialEq)]
pub(crate) enum HookAction {
    /// Changes, in place, the step data it received under this key.
    ChangeStepData {
        key: String,
        value: Value,
    },
    Contribute {
        key: String,
        value: Value,
    },
    /// Adds 1 to a counter its policy instance holds, from 0, and contributes the count.
    ContributeCallCount {
        key: String,
    },
    Emit {
        level: Level,
        message: String,
    },
    CallRole {
        role: String,
        operation: String,
    },
}

impl HookAction {
    /// The value this action gives the key: what it contributes or writes under it.
    pub(crate) fn value_for(&self, key: &str) -> Option<&Value> {
        match self {
            Self::ChangeStepData { key: name, value } | Self::Contribute { key: name, value }
                if name == key =>
            {
                Some(value)
            }
            _ => None,
        }
    }
}

/// How a hook ends.
#[derive(Debug, Default, PartialEq)]
pub(crate) enum HookReturn {
    /// No lifecycle.
    #[default]
    Nothing,
    FinishWorkflow,
    FailWorkflow {
        code: String,
    },
    /// The hook fails, with this message when one is given.
    Fails(Option<String>),
}

/// A role the workflow provides: its operations, and the ones that fail when called.
#[derive(Debug, Default)]
pub(crate) struct Role {
    pub(crate) operations: BTreeSet<String>,
    pub(crate) failing: BTreeSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hook_is_read_by_its_name() {
        for hook in Hook::ALL {
            assert_eq!(Hook::named(&hook.to_string()).unwrap(), hook);
        }
    }

    #[test]
    fn a_name_outside_the_hooks_is_refused() {
        assert!(Hook::named("on step start").is_err());
    }

    #[test]
    fn a_policy_holds_only_hooks_of_its_own_kind() {
        let mut policy = Policy::new(Hook::Step(StepHook::OnStepSuccess));
        assert!(
            policy
                .define("audit", Hook::Step(StepHook::OnStepFailure))
                .is_ok()
        );
        assert_eq!(
            policy
                .define("audit", Hook::Workflow(WorkflowHook::OnWorkflowSuccess))
                .unwrap_err(),
            ModelError::HookOfOtherKind(
                "audit".to_owned(),
                Hook::Workflow(WorkflowHook::OnWorkflowSuccess)
            )
        );
        assert_eq!(
            policy
                .define("audit", Hook::Step(StepHook::OnStepSuccess))
                .unwrap_err(),
            ModelError::HookDefinedTwice("audit".to_owned(), Hook::Step(StepHook::OnStepSuccess))
        );
    }
}
