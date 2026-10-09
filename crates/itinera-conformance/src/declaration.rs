//! The scenario's workflow, declared with itinera's builder from the scenario model.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use itinera::error::Error;
use itinera::policy::{StepPolicyDescriptor, WorkflowPolicyDescriptor};
use itinera::step::{Outcome, StepDescriptor, StepName};
use itinera::workflow::{InputAdapter, Violations, WorkflowBuilder, WorkflowDescriptor};

use crate::model::{Adapter, HookScript, Hooks, Model, ModelError, Policy, Workflow};

/// The workflow's own type.
#[derive(Debug)]
pub(crate) struct ScriptedWorkflow;

/// What building the scenario's workflow gave.
#[derive(Debug)]
pub(crate) enum Admission {
    Admitted(WorkflowDescriptor<ScriptedWorkflow>),
    Refused(Violations),
}

/// How many times the scenario's steps ran, whatever step it was.
#[derive(Clone, Debug, Default)]
pub(crate) struct StepsRun {
    count: Arc<AtomicUsize>,
}

impl StepsRun {
    pub(crate) fn none(&self) -> bool {
        self.count.load(Ordering::SeqCst) == 0
    }

    /// Counts one more run of a step, which succeeds.
    fn run(&self) -> Result<Outcome, Error> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(Outcome::success())
    }
}

/// Declares the scenario's workflow, whose steps count their runs in `steps_run`.
pub(crate) fn declared(
    model: &Model,
    steps_run: &StepsRun,
) -> Result<WorkflowBuilder<ScriptedWorkflow>, ModelError> {
    let workflow = model.workflow()?;
    let steps = workflow
        .steps
        .iter()
        .map(|step| step_descriptor(model, workflow, step, steps_run))
        .collect::<Result<Vec<_>, _>>()?;
    let policies = workflow
        .policies
        .iter()
        .map(|policy| workflow_policy(model, policy))
        .collect::<Result<Vec<_>, _>>()?;
    let adapters = workflow
        .adapters
        .iter()
        .map(input_adapter)
        .collect::<Result<Vec<_>, _>>()?;
    let builder = WorkflowDescriptor::builder(leaked(&workflow.name));
    let builder = steps.into_iter().fold(builder, WorkflowBuilder::step);
    let builder = policies.into_iter().fold(builder, WorkflowBuilder::policy);
    Ok(adapters
        .into_iter()
        .fold(builder, WorkflowBuilder::input_adapter))
}

fn step_descriptor(
    model: &Model,
    workflow: &Workflow,
    step: &str,
    steps_run: &StepsRun,
) -> Result<StepDescriptor, ModelError> {
    let runs = steps_run.clone();
    let descriptor = StepDescriptor::new(step_name(step)?, move || runs.run());
    let policies = workflow
        .step_policies
        .get(step)
        .into_iter()
        .flatten()
        .map(|policy| step_policy(model, policy))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(policies
        .into_iter()
        .fold(descriptor, StepDescriptor::policy))
}

fn step_policy(model: &Model, name: &str) -> Result<StepPolicyDescriptor, ModelError> {
    match &policy(model, name)?.hooks {
        Hooks::Step(hooks) => {
            let ((first, _), rest) = hooks.split_first().ok_or_else(|| no_hooks(name))?;
            Ok(rest.iter().map(hook_of).fold(
                StepPolicyDescriptor::new(leaked(name), *first),
                StepPolicyDescriptor::hook,
            ))
        }
        Hooks::Workflow(_) => Err(ModelError::NotStepPolicy(name.to_owned())),
    }
}

fn workflow_policy(model: &Model, name: &str) -> Result<WorkflowPolicyDescriptor, ModelError> {
    match &policy(model, name)?.hooks {
        Hooks::Workflow(hooks) => {
            let ((first, _), rest) = hooks.split_first().ok_or_else(|| no_hooks(name))?;
            Ok(rest.iter().map(hook_of).fold(
                WorkflowPolicyDescriptor::new(leaked(name), *first),
                WorkflowPolicyDescriptor::hook,
            ))
        }
        Hooks::Step(_) => Err(ModelError::NotWorkflowPolicy(name.to_owned())),
    }
}

fn policy<'a>(model: &'a Model, name: &str) -> Result<&'a Policy, ModelError> {
    model
        .policies
        .get(name)
        .ok_or_else(|| ModelError::UnknownPolicy(name.to_owned()))
}

fn hook_of<H: Copy>((hook, _): &(H, HookScript)) -> H {
    *hook
}

fn no_hooks(policy: &str) -> ModelError {
    ModelError::NoHooks(policy.to_owned())
}

fn input_adapter(adapter: &Adapter) -> Result<InputAdapter<ScriptedWorkflow>, ModelError> {
    let steps = adapter
        .steps
        .iter()
        .map(String::as_str)
        .map(step_name)
        .collect::<Result<Vec<_>, _>>()?;
    let (first, rest) = steps
        .split_first()
        .ok_or_else(|| ModelError::AdapterWithoutSteps(adapter.name.clone()))?;
    Ok(rest.iter().copied().fold(
        InputAdapter::new(leaked(&adapter.name), *first, |_, _, _| Ok(None)),
        InputAdapter::step,
    ))
}

/// A step name, which may not be empty.
fn step_name(name: &str) -> Result<StepName, ModelError> {
    if name.is_empty() {
        Err(ModelError::EmptyStepName)
    } else {
        Ok(StepName::new(leaked(name)))
    }
}

/// The name, for as long as the runner runs.
///
/// Applications name their workflow's parts with constants. The runner reads the names from the
/// cases, so it leaks each one, a few short strings for each scenario.
fn leaked(name: &str) -> &'static str {
    Box::leak(Box::from(name))
}

#[cfg(test)]
mod tests {
    use itinera::workflow::{Violation, ViolationKind};

    use super::*;

    fn model() -> Model {
        let mut model = Model::default();
        model
            .declare(
                "orders".to_owned(),
                vec!["charge".to_owned(), "ship".to_owned()],
            )
            .unwrap();
        model
    }

    #[test]
    fn the_scenarios_steps_policies_and_adapters_are_declared_in_order() {
        let mut model = model();
        model
            .define("audit".to_owned(), "on step success", true)
            .unwrap();
        model
            .define("alarm".to_owned(), "on step failure", true)
            .unwrap();
        model
            .define("notify".to_owned(), "on workflow success", false)
            .unwrap();
        let workflow = model.workflow_mut().unwrap();
        workflow.step_policies.insert(
            "charge".to_owned(),
            vec!["audit".to_owned(), "alarm".to_owned()],
        );
        workflow.policies.push("notify".to_owned());
        workflow.adapters.push(Adapter {
            name: "pricing".to_owned(),
            steps: vec!["ship".to_owned()],
            answers: Vec::new(),
            requests: Vec::new(),
        });

        let descriptor = declared(&model, &StepsRun::default())
            .unwrap()
            .build()
            .unwrap();

        let listing = descriptor.listing();
        assert_eq!(listing[0].step, StepName::new("charge"));
        assert_eq!(listing[1].step, StepName::new("ship"));
        let policies: Vec<String> = listing[0]
            .policies
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(policies, ["audit", "alarm"]);
        let adapter = listing[1].adapter.as_ref().map(ToString::to_string);
        assert_eq!(adapter.as_deref(), Some("pricing"));
    }

    #[test]
    fn a_step_policy_attached_twice_defines_its_hooks_twice() {
        let mut model = model();
        model
            .define("audit".to_owned(), "on step success", true)
            .unwrap();
        model.workflow_mut().unwrap().step_policies.insert(
            "charge".to_owned(),
            vec!["audit".to_owned(), "audit".to_owned()],
        );

        let violations = declared(&model, &StepsRun::default())
            .unwrap()
            .build()
            .unwrap_err();

        let kinds: Vec<ViolationKind> = violations.iter().map(Violation::kind).collect();
        assert_eq!(kinds, [ViolationKind::HookDefinedTwice]);
    }

    #[test]
    fn declaring_the_workflow_runs_no_step() {
        let steps_run = StepsRun::default();
        let _ = declared(&model(), &steps_run).unwrap().build().unwrap();
        assert!(steps_run.none());
    }

    #[test]
    fn an_empty_step_name_is_a_case_error() {
        let mut model = Model::default();
        model
            .declare("orders".to_owned(), vec![String::new()])
            .unwrap();

        let declared = declared(&model, &StepsRun::default());

        assert_eq!(declared.err(), Some(ModelError::EmptyStepName));
    }
}
