//! The scenario's workflow, declared with itinera's builder from the scenario model, in the
//! mode of the executor that runs it.

use itinera::error::Error;
use itinera::journey::DataBag;
use itinera::mode::{Asynchronous, Mode, Synchronous};
use itinera::policy::{
    HookNeeds, Hooked, Hookless, InputAdapter, Provides, Requested, StepHook, StepPolicyDescriptor,
    WorkflowHook, WorkflowPolicyDescriptor,
};
use itinera::step::{StepDescriptor, StepName};
use itinera::value::AnyValue;
use itinera::workflow::{
    InputAdapterDescriptor, ListedStep, Violations, WorkflowBuilder, WorkflowDescriptor,
};

use crate::model::{
    Adapter, Answer, HookScript, Hooks, IdGenerator, Model, ModelError, Policy, Workflow,
};
use crate::policy::{Building, ScriptedPolicy, Scripts, requiring_from_workflow};
use crate::record::{ListedReporter, Recorder};
use crate::role::{ProvidedRoles, Roles};
use crate::step::{Scripted, StepsRun};
use crate::value::Typed;
use crate::witness::Witness;

/// The workflow's own value: the recorders of the reporters it lists, in order, which its
/// instance hands its reporters as it makes them, and the roles it provides.
#[derive(Debug)]
pub(crate) struct ScriptedWorkflow {
    reporters: Vec<Recorder>,
    roles: ProvidedRoles,
}

impl Provides<dyn Roles> for ScriptedWorkflow {
    fn role(&self) -> &(dyn Roles + 'static) {
        &self.roles
    }
}

impl ScriptedWorkflow {
    pub(crate) fn new(reporters: Vec<Recorder>, roles: ProvidedRoles) -> Self {
        Self { reporters, roles }
    }

    /// The recorder of the reporter listed at this position, counted from 0.
    pub(crate) fn reporter(&self, position: usize) -> Option<&Recorder> {
        self.reporters.get(position)
    }
}

/// What building the scenario's workflow gave.
#[derive(Debug)]
pub(crate) enum Admission {
    /// The workflow was admitted, with this listing.
    Admitted(Vec<ListedStep>),
    Refused(Violations),
}

/// A scripted step policy, declared in the mode `M`, which names its hooks once it is `Hooked`.
type StepPolicy<M, S = Hooked> = StepPolicyDescriptor<ScriptedPolicy, ScriptedWorkflow, M, S>;

/// A scripted workflow policy, declared in the mode `M`, which names its hooks once it is
/// `Hooked`.
type WorkflowPolicy<M, S = Hooked> =
    WorkflowPolicyDescriptor<ScriptedPolicy, ScriptedWorkflow, M, S>;

/// An execution mode the scenario's workflow can be declared in.
pub(crate) trait Declares: Mode + Sized {
    fn builder(name: &'static str) -> WorkflowBuilder<ScriptedWorkflow, Self>;

    fn step(name: StepName, factory: Scripted) -> StepDescriptor<ScriptedWorkflow, Self>;

    fn step_policy(building: Building) -> StepPolicy<Self, Hookless>;

    /// The step policy, which defines this hook too, needing what its script says.
    fn step_hook<S>(
        policy: StepPolicy<Self, S>,
        hook: StepHook,
        scripts: &Scripts,
    ) -> Result<StepPolicy<Self>, ModelError>;

    fn workflow_policy(building: Building) -> WorkflowPolicy<Self, Hookless>;

    /// The workflow policy, which defines this hook too, needing what its script says.
    fn workflow_hook<S>(
        policy: WorkflowPolicy<Self, S>,
        hook: WorkflowHook,
        scripts: &Scripts,
    ) -> Result<WorkflowPolicy<Self>, ModelError>;
}

impl Declares for Synchronous {
    fn builder(name: &'static str) -> WorkflowBuilder<ScriptedWorkflow, Self> {
        WorkflowDescriptor::builder(name)
    }

    fn step(name: StepName, factory: Scripted) -> StepDescriptor<ScriptedWorkflow, Self> {
        StepDescriptor::new(name, factory)
    }

    fn step_policy(building: Building) -> StepPolicy<Self, Hookless> {
        StepPolicyDescriptor::fallible(building.name(), move || building.build())
    }

    fn step_hook<S>(
        policy: StepPolicy<Self, S>,
        hook: StepHook,
        scripts: &Scripts,
    ) -> Result<StepPolicy<Self>, ModelError> {
        Ok(match hook {
            StepHook::OnStepSuccess => policy.on_step_success_needing(scripts.needs()?),
            StepHook::OnStepFailure => policy.on_step_failure_needing(scripts.needs()?),
            StepHook::OnStepRetry => policy.on_step_retry_needing(scripts.needs()?),
            StepHook::OnStepAbnormalTermination => {
                policy.on_step_abnormal_termination_needing(scripts.needs()?)
            }
            _ => return Err(unknown(hook)),
        })
    }

    fn workflow_policy(building: Building) -> WorkflowPolicy<Self, Hookless> {
        WorkflowPolicyDescriptor::fallible(building.name(), move || building.build())
    }

    fn workflow_hook<S>(
        policy: WorkflowPolicy<Self, S>,
        hook: WorkflowHook,
        scripts: &Scripts,
    ) -> Result<WorkflowPolicy<Self>, ModelError> {
        Ok(match hook {
            WorkflowHook::OnWorkflowSuccess => policy.on_workflow_success_needing(scripts.needs()?),
            WorkflowHook::OnWorkflowFailure => policy.on_workflow_failure_needing(scripts.needs()?),
            _ => return Err(unknown(hook)),
        })
    }
}

impl Declares for Asynchronous {
    fn builder(name: &'static str) -> WorkflowBuilder<ScriptedWorkflow, Self> {
        WorkflowDescriptor::async_builder(name)
    }

    fn step(name: StepName, factory: Scripted) -> StepDescriptor<ScriptedWorkflow, Self> {
        StepDescriptor::new_async(name, factory)
    }

    fn step_policy(building: Building) -> StepPolicy<Self, Hookless> {
        StepPolicyDescriptor::fallible_async(building.name(), move || building.build())
    }

    fn step_hook<S>(
        policy: StepPolicy<Self, S>,
        hook: StepHook,
        scripts: &Scripts,
    ) -> Result<StepPolicy<Self>, ModelError> {
        Ok(match hook {
            StepHook::OnStepSuccess => policy.on_step_success_needing(scripts.needs()?),
            StepHook::OnStepFailure => policy.on_step_failure_needing(scripts.needs()?),
            StepHook::OnStepRetry => policy.on_step_retry_needing(scripts.needs()?),
            StepHook::OnStepAbnormalTermination => {
                policy.on_step_abnormal_termination_needing(scripts.needs()?)
            }
            _ => return Err(unknown(hook)),
        })
    }

    fn workflow_policy(building: Building) -> WorkflowPolicy<Self, Hookless> {
        WorkflowPolicyDescriptor::fallible_async(building.name(), move || building.build())
    }

    fn workflow_hook<S>(
        policy: WorkflowPolicy<Self, S>,
        hook: WorkflowHook,
        scripts: &Scripts,
    ) -> Result<WorkflowPolicy<Self>, ModelError> {
        Ok(match hook {
            WorkflowHook::OnWorkflowSuccess => policy.on_workflow_success_needing(scripts.needs()?),
            WorkflowHook::OnWorkflowFailure => policy.on_workflow_failure_needing(scripts.needs()?),
            _ => return Err(unknown(hook)),
        })
    }
}

/// A hook itinera knows that the runner does not.
fn unknown(hook: impl ToString) -> ModelError {
    ModelError::UnknownHook(hook.to_string())
}

/// Declares the scenario's workflow, whose steps count their runs in `steps_run`, and whose
/// steps and hooks record what they received in `witness`.
pub(crate) fn declared<M: Declares>(
    model: &Model,
    steps_run: &StepsRun,
    witness: &Witness,
) -> Result<WorkflowBuilder<ScriptedWorkflow, M>, ModelError> {
    let workflow = model.workflow()?;
    let steps = workflow
        .steps
        .iter()
        .map(|step| step_descriptor(model, workflow, step, steps_run, witness))
        .collect::<Result<Vec<_>, _>>()?;
    let policies = workflow
        .policies
        .iter()
        .map(|policy| workflow_policy::<M>(model, policy, witness))
        .collect::<Result<Vec<_>, _>>()?;
    let adapters = workflow
        .adapters
        .iter()
        .map(input_adapter)
        .collect::<Result<Vec<_>, _>>()?;
    let lists = reporter_lists::<M>();
    if workflow.reporters.len() > lists.len() {
        return Err(ModelError::TooManyReporters(lists.len()));
    }
    let builder = M::builder(leaked(&workflow.name));
    let builder = steps.into_iter().fold(builder, WorkflowBuilder::step);
    let builder = policies.into_iter().fold(builder, WorkflowBuilder::policy);
    let builder = adapters
        .into_iter()
        .fold(builder, WorkflowBuilder::input_adapter);
    let builder = lists
        .into_iter()
        .take(workflow.reporters.len())
        .fold(builder, |builder, list| list(builder));
    Ok(match &workflow.id_generator {
        IdGenerator::Default => builder,
        IdGenerator::Returns(id) => builder.id_generator(returning(id.clone())),
    })
}

/// An ID generator that always returns this ID.
fn returning(id: String) -> impl Fn(&ScriptedWorkflow, &DataBag) -> Result<String, Error> {
    move |_, _| Ok(id.clone())
}

fn step_descriptor<M: Declares>(
    model: &Model,
    workflow: &Workflow,
    step: &str,
    steps_run: &StepsRun,
    witness: &Witness,
) -> Result<StepDescriptor<ScriptedWorkflow, M>, ModelError> {
    let script = workflow
        .scripts
        .get(step)
        .ok_or_else(|| ModelError::UnknownStep(step.to_owned()))?;
    let scripted = Scripted::new(step, script, steps_run.clone(), witness.clone())?;
    let descriptor = M::step(step_name(step)?, scripted).retry_budget(script.retries);
    let descriptor = if script.abnormal_termination_retriable {
        descriptor.abnormal_termination_retriable()
    } else {
        descriptor
    };
    let policies = workflow
        .step_policies
        .get(step)
        .into_iter()
        .flatten()
        .map(|policy| step_policy::<M>(model, policy, witness))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(policies
        .into_iter()
        .fold(descriptor, StepDescriptor::policy))
}

fn step_policy<M: Declares>(
    model: &Model,
    name: &str,
    witness: &Witness,
) -> Result<StepPolicy<M>, ModelError> {
    let policy = policy(model, name)?;
    let Hooks::Step(hooks) = &policy.hooks else {
        return Err(ModelError::NotStepPolicy(name.to_owned()));
    };
    let building = Building::of(name, policy, witness)?;
    let scripts = building.scripts();
    let mut hooks = hooks.iter().map(hook_of);
    let first = hooks.next().ok_or_else(|| no_hooks(name))?;
    let declared = M::step_hook(M::step_policy(building), first, &scripts)?;
    hooks.try_fold(declared, |declared, hook| {
        M::step_hook(declared, hook, &scripts)
    })
}

fn workflow_policy<M: Declares>(
    model: &Model,
    name: &str,
    witness: &Witness,
) -> Result<WorkflowPolicy<M>, ModelError> {
    let policy = policy(model, name)?;
    let Hooks::Workflow(hooks) = &policy.hooks else {
        return Err(ModelError::NotWorkflowPolicy(name.to_owned()));
    };
    let building = Building::of(name, policy, witness)?;
    let scripts = building.scripts();
    let mut hooks = hooks.iter().map(hook_of);
    let first = hooks.next().ok_or_else(|| no_hooks(name))?;
    let declared = M::workflow_hook(M::workflow_policy(building), first, &scripts)?;
    hooks.try_fold(declared, |declared, hook| {
        M::workflow_hook(declared, hook, &scripts)
    })
}

fn policy<'a>(model: &'a Model, name: &str) -> Result<&'a Policy, ModelError> {
    model
        .policies
        .get(name)
        .ok_or_else(|| ModelError::UnknownPolicy(name.to_owned()))
}

/// A policy of the scenario that defines no hook, which itinera cannot declare.
fn no_hooks(name: &str) -> ModelError {
    ModelError::NoHooks(name.to_owned())
}

fn hook_of<H: Copy>((hook, _): &(H, HookScript)) -> H {
    *hook
}

fn input_adapter(
    adapter: &Adapter,
) -> Result<InputAdapterDescriptor<ScriptedWorkflow>, ModelError> {
    let steps = adapter
        .steps
        .iter()
        .map(String::as_str)
        .map(step_name)
        .collect::<Result<Vec<_>, _>>()?;
    let (first, rest) = steps
        .split_first()
        .ok_or_else(|| ModelError::AdapterWithoutSteps(adapter.name.clone()))?;
    let answers = adapter
        .answers
        .iter()
        .map(Answering::of)
        .collect::<Result<Vec<_>, _>>()?;
    let needs = adapter
        .requests
        .iter()
        .fold(HookNeeds::new(), requiring_from_workflow);
    Ok(rest.iter().copied().fold(
        InputAdapterDescriptor::new(leaked(&adapter.name), *first, move |_, got| {
            answer(&answers, &got)
        })
        .needing(needs),
        InputAdapterDescriptor::step,
    ))
}

/// A scripted adapter's answer for one key, with its value typed.
#[derive(Debug)]
struct Answering {
    key: String,
    answer: Answered,
}

#[derive(Debug)]
enum Answered {
    Value(Typed),
    Nothing,
    Fails(String),
}

impl Answering {
    fn of((key, answer): &(String, Answer)) -> Result<Self, ModelError> {
        let answer = match answer {
            Answer::Value(value) => Answered::Value(Typed::of(value)?),
            Answer::Nothing => Answered::Nothing,
            Answer::Fails(message) => Answered::Fails(message.clone()),
        };
        Ok(Self {
            key: key.clone(),
            answer,
        })
    }

    fn is_for(&self, key: &str) -> bool {
        self.key == key
    }
}

/// What the adapter answers for the key it is called for: nothing for a key its script does not
/// mention.
fn answer(
    answers: &[Answering],
    got: &Requested<'_, ScriptedWorkflow, InputAdapter>,
) -> Result<Option<AnyValue>, Error> {
    let key = got.key();
    let answered = answers
        .iter()
        .find(|answering| answering.is_for(key))
        .map(|answering| &answering.answer);
    match answered {
        Some(Answered::Value(value)) => Ok(Some(value.clone().erased())),
        Some(Answered::Fails(message)) => Err(Error::msg(message.clone())),
        Some(Answered::Nothing) | None => Ok(None),
    }
}

/// A builder step that lists one more reporter.
type Lists<M> = fn(WorkflowBuilder<ScriptedWorkflow, M>) -> WorkflowBuilder<ScriptedWorkflow, M>;

/// Lists the reporter at each position, up to as many as the runner can list: each position is
/// a type of its own, since the builder lists a reporter by its type.
fn reporter_lists<M: Mode>() -> [Lists<M>; 4] {
    [list::<0, M>, list::<1, M>, list::<2, M>, list::<3, M>]
}

fn list<const POSITION: usize, M: Mode>(
    builder: WorkflowBuilder<ScriptedWorkflow, M>,
) -> WorkflowBuilder<ScriptedWorkflow, M> {
    builder.reporter::<ListedReporter<POSITION>>()
}

/// A step name, which may not be empty.
fn step_name(name: &str) -> Result<StepName, ModelError> {
    if name.is_empty() {
        Err(ModelError::EmptyStepName)
    } else {
        Ok(StepName::new(leaked(name)))
    }
}

/// The name or key, for as long as the runner runs.
///
/// Applications name their workflow's parts and their steps' inputs with constants. The runner
/// reads them from the cases, so it leaks each one, a few short strings for each scenario.
pub(crate) fn leaked(name: &str) -> &'static str {
    Box::leak(Box::from(name))
}

#[cfg(test)]
mod tests {
    use itinera::workflow::{Violation, ViolationKind};

    use super::*;

    /// The scenario's workflow, declared synchronous, with nothing counting its steps' runs.
    fn declared_synchronously(
        model: &Model,
    ) -> Result<WorkflowBuilder<ScriptedWorkflow, Synchronous>, ModelError> {
        declared(model, &StepsRun::default(), &Witness::default())
    }

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

        let descriptor = declared_synchronously(&model).unwrap().build().unwrap();

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

        let violations = declared_synchronously(&model).unwrap().build().unwrap_err();

        let kinds: Vec<ViolationKind> = violations.iter().map(Violation::kind).collect();
        assert_eq!(kinds, [ViolationKind::HookDefinedTwice]);
    }

    #[test]
    fn declaring_the_workflow_runs_no_step() {
        let steps_run = StepsRun::default();
        let _ = declared::<Synchronous>(&model(), &steps_run, &Witness::default())
            .unwrap()
            .build()
            .unwrap();
        assert!(steps_run.none());
    }

    #[test]
    fn an_empty_step_name_is_a_case_error() {
        let mut model = Model::default();
        model
            .declare("orders".to_owned(), vec![String::new()])
            .unwrap();

        let declared = declared_synchronously(&model);

        assert_eq!(declared.err(), Some(ModelError::EmptyStepName));
    }
}
