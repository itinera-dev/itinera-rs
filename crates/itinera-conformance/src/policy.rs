//! The scenario's scripted policies: what each hook requests, does and returns, as the scenario
//! says, in the mode of the executor that runs them.

use std::future::{Future, ready};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use futures::executor::block_on;
use itinera::error::Error;
use itinera::journey::Contributor;
use itinera::mode::{Asynchronous, Synchronous};
use itinera::policy::{
    AsyncHookReporter, AsyncOnStepAbnormalTermination, AsyncOnStepFailure, AsyncOnStepRetry,
    AsyncOnStepSuccess, AsyncOnWorkflowFailure, AsyncOnWorkflowSuccess, ErrorHookKind,
    FailWorkflow, FailureHookKind, HookKind, HookNeeds, HookReporter, OnStepAbnormalTermination,
    OnStepFailure, OnStepRetry, OnStepSuccess, OnSuccess, OnWorkflowFailure, OnWorkflowSuccess,
    PolicyHookKind, Requested, StepAbnormalTermination, StepFailure, StepHook, StepHookKind,
    StepRetry, StepSuccess, WorkflowFailure, WorkflowHook, WorkflowSuccess,
};
use itinera::step::{Input, OptionalInput, Reason};
use itinera::value::Value as Storable;
use serde_json::Value;

use crate::declaration::{ScriptedWorkflow, leaked};
use crate::model::{
    self, Hook, HookAction, HookRequest, HookReturn, HookScript, Hooks, Level, Lifecycle,
    ModelError, ValueType,
};
use crate::role::Roles;
use crate::step::{Emitted, contribute};
use crate::value::{Data, ForType, Typed, for_type, json};
use crate::witness::{Call, Received, Witness};

/// An instance of one of the scenario's policies, of either kind, built for an attempt or a
/// journey; it does what the scripts of its hooks say.
#[derive(Debug)]
pub(crate) struct ScriptedPolicy {
    name: &'static str,
    scripts: Arc<Scripts>,
    /// The counter its hooks add their calls to, from 0 in each instance.
    count: AtomicI64,
    witness: Witness,
}

impl ScriptedPolicy {
    fn new(name: &'static str, scripts: Arc<Scripts>, witness: Witness) -> Self {
        Self {
            name,
            scripts,
            count: AtomicI64::new(0),
            witness,
        }
    }

    /// Takes what the hook requested, then does what its script says and returns what it says.
    async fn run<'a, H: Scripting, M, R: Emits>(
        &self,
        mut got: Requested<'a, ScriptedWorkflow, H, M>,
        reporter: fn(&mut Requested<'a, ScriptedWorkflow, H, M>) -> Result<R, Error>,
    ) -> Result<H::Returns, Error> {
        let script = H::script(&self.scripts).ok_or_else(unscripted::<H>)?;
        let mut received = Received::default();
        script
            .requests
            .iter()
            .try_for_each(|request| H::receive(&mut got, request, &mut received))?;
        self.witness.called(Call {
            policy: self.name.to_owned(),
            hook: H::HOOK,
            received,
        });
        let roles = got.role::<dyn Roles>();
        let mut contributor = got.contributor()?;
        let mut reporter = reporter(&mut got)?;
        for action in &script.actions {
            self.perform(action, roles, &mut contributor, &mut reporter)
                .await?;
        }
        script.returns.clone().map_err(Error::msg)
    }

    async fn perform<R: Emits>(
        &self,
        action: &Action,
        roles: &dyn Roles,
        contributor: &mut Contributor<'_>,
        reporter: &mut R,
    ) -> Result<(), Error> {
        match action {
            Action::Contribute(key, value) => contribute(contributor, key, value),
            Action::CountCalls(key) => {
                let count = self.count.fetch_add(1, Ordering::SeqCst) + 1;
                contributor.contribute(key, count);
            }
            Action::Emit(level, message) => reporter.emit(*level, message.clone()).await?,
            Action::CallRole { role, operation } => roles.call(role, operation)?,
        }
        Ok(())
    }
}

/// The error of a hook called without a script, which only a hook the policy does not define
/// lacks, and such a hook is never declared.
fn unscripted<H: Scripting>() -> Error {
    Error::msg(format!("the hook {} has no script", H::HOOK))
}

/// How one of the scenario's policies is built: what its hooks do, and whether building it
/// fails, with this message.
#[derive(Debug)]
pub(crate) struct Building {
    name: &'static str,
    scripts: Arc<Scripts>,
    failure: Option<String>,
    witness: Witness,
}

impl Building {
    pub(crate) fn of(
        name: &str,
        policy: &model::Policy,
        witness: &Witness,
    ) -> Result<Self, ModelError> {
        Ok(Self {
            name: leaked(name),
            scripts: Arc::new(Scripts::of(name, policy)?),
            failure: policy.construction_failure.clone().map(failure_message),
            witness: witness.clone(),
        })
    }

    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn scripts(&self) -> Arc<Scripts> {
        Arc::clone(&self.scripts)
    }

    /// A new instance of the policy, unless building it fails.
    pub(crate) fn build(&self) -> Result<ScriptedPolicy, Error> {
        match &self.failure {
            Some(message) => Err(Error::msg(message.clone())),
            None => Ok(ScriptedPolicy::new(
                self.name,
                self.scripts(),
                self.witness.clone(),
            )),
        }
    }
}

fn failure_message(message: Option<String>) -> String {
    message.unwrap_or_else(|| "the scripted policy cannot be built".to_owned())
}

/// What the scripts of a policy's hooks say, each typed for its hook, and the name of the
/// policy; a hook it does not define has no script.
#[derive(Debug, Default)]
pub(crate) struct Scripts {
    policy: String,
    on_step_success: Option<Script<StepSuccess>>,
    on_step_failure: Option<Script<StepFailure>>,
    on_step_retry: Option<Script<StepRetry>>,
    on_step_abnormal_termination: Option<Script<StepAbnormalTermination>>,
    on_workflow_success: Option<Script<WorkflowSuccess>>,
    on_workflow_failure: Option<Script<WorkflowFailure>>,
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
    requests: Vec<Request>,
    actions: Vec<Action>,
    returns: Returning<H::Returns>,
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

/// What a hook requests, with its keys as itinera takes them. A role is not requested: a hook
/// reaches it when it calls it.
///
/// It displays as an error names it.
#[derive(Debug, derive_more::Display)]
pub(crate) enum Request {
    #[display("step data")]
    StepData(Data),
    #[display("data from the workflow")]
    WorkflowData(Data),
    #[display("the step name")]
    StepName,
    #[display("the attempt number")]
    Attempt,
    #[display("the failure cause")]
    FailureCause,
    #[display("the retry cause")]
    RetryCause,
    /// The failure reason, optional or required.
    #[display("the failure reason")]
    Reason { optional: bool },
    #[display("the error")]
    Error,
    #[display("the journey ID")]
    JourneyId,
}

impl Request {
    fn of(request: &HookRequest) -> Option<Self> {
        Some(match request {
            HookRequest::StepData {
                key,
                value_type,
                optional,
            } => Self::StepData(Data::of(key, *value_type, *optional)),
            HookRequest::DataFromWorkflow {
                key,
                value_type,
                optional,
            } => Self::WorkflowData(Data::of(key, *value_type, *optional)),
            HookRequest::StepName => Self::StepName,
            HookRequest::AttemptNumber => Self::Attempt,
            HookRequest::FailureCause => Self::FailureCause,
            HookRequest::RetryCause => Self::RetryCause,
            HookRequest::FailureReason { optional } => Self::Reason {
                optional: *optional,
            },
            HookRequest::Error => Self::Error,
            HookRequest::JourneyId => Self::JourneyId,
            HookRequest::Role(_) => return None,
        })
    }
}

/// What a hook does before it returns, with its values typed.
///
/// Rust gives a hook its own copy of what it receives, so a hook that changes it changes
/// nothing anyone else sees. The script leaves those changes out.
#[derive(Debug)]
enum Action {
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

/// How a hook ends: with what it returns, or failing with this message.
pub(crate) type Returning<R> = Result<R, String>;

/// A hook that fails, with the message the scenario gives, if any.
fn fails<R>(message: Option<&str>) -> Returning<R> {
    Err(message.map_or_else(|| "the scripted hook fails".to_owned(), str::to_owned))
}

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

/// What a hook that may fail the workflow returns.
fn failing_returning(
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
fn workflow_returning(hook: Hook, returned: &HookReturn) -> Result<Returning<()>, ModelError> {
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

/// Declares a request every hook called because an attempt did not succeed may make.
fn declare_for_failure<H: FailureHookKind + ErrorHookKind>(
    needs: HookNeeds<H>,
    request: &Request,
) -> Result<HookNeeds<H>, String> {
    match request {
        Request::Reason { optional: false } => Ok(needs.reason()),
        Request::Reason { optional: true } => Ok(needs.optional_reason()),
        _ => declare_for_error(needs, request),
    }
}

/// Declares a request every hook that may request the error may make.
fn declare_for_error<H: ErrorHookKind>(
    needs: HookNeeds<H>,
    request: &Request,
) -> Result<HookNeeds<H>, String> {
    match request {
        Request::Error => Ok(needs.error()),
        _ => declare_for_step(needs, request),
    }
}

/// Declares a request every step hook may make.
fn declare_for_step<H: StepHookKind>(
    needs: HookNeeds<H>,
    request: &Request,
) -> Result<HookNeeds<H>, String> {
    match request {
        Request::StepData(data) => Ok(for_type(data.value_type, NeedingFromStep { needs, data })),
        Request::StepName | Request::Attempt => Ok(needs),
        _ => declare_for_any(needs, request),
    }
}

/// Declares a request every hook may make.
fn declare_for_any<H: HookKind>(
    needs: HookNeeds<H>,
    request: &Request,
) -> Result<HookNeeds<H>, String> {
    match request {
        Request::WorkflowData(data) => Ok(for_type(
            data.value_type,
            NeedingFromWorkflow { needs, data },
        )),
        Request::JourneyId => Ok(needs),
        _ => Err(request.to_string()),
    }
}

/// Takes what a hook called because an attempt did not succeed received for a request every
/// such hook may make.
fn receive_for_failure<H: FailureHookKind + ErrorHookKind, M>(
    got: &mut Requested<'_, ScriptedWorkflow, H, M>,
    request: &Request,
    received: &mut Received,
) -> Result<(), Error> {
    match request {
        Request::Reason { optional } => received.reason = Some(reason(got, *optional)?),
        _ => return receive_for_error(got, request, received),
    }
    Ok(())
}

/// Takes what a hook that may request the error received for a request every such hook may
/// make.
fn receive_for_error<H: ErrorHookKind, M>(
    got: &mut Requested<'_, ScriptedWorkflow, H, M>,
    request: &Request,
    received: &mut Received,
) -> Result<(), Error> {
    match request {
        Request::Error => received.error = Some(got.error()?.to_string()),
        _ => return receive_for_step(got, request, received),
    }
    Ok(())
}

/// Takes what a step hook received for a request every step hook may make.
fn receive_for_step<H: StepHookKind, M>(
    got: &mut Requested<'_, ScriptedWorkflow, H, M>,
    request: &Request,
    received: &mut Received,
) -> Result<(), Error> {
    match request {
        Request::StepData(data) => {
            let value = for_type(data.value_type, TakingFromStep { got, data })?;
            received.step_data.insert(data.key.to_owned(), value);
        }
        Request::StepName => received.step_name = Some(got.step_name().to_string()),
        Request::Attempt => received.attempt = Some(got.attempt()),
        _ => return receive_for_any(got, request, received),
    }
    Ok(())
}

/// Takes what a hook received for a request every hook may make.
fn receive_for_any<H: HookKind, M>(
    got: &mut Requested<'_, ScriptedWorkflow, H, M>,
    request: &Request,
    received: &mut Received,
) -> Result<(), Error> {
    match request {
        Request::WorkflowData(data) => {
            let value = for_type(data.value_type, TakingFromWorkflow { got, data })?;
            received.workflow_data.insert(data.key.to_owned(), value);
        }
        Request::JourneyId => received.journey_id = Some(got.journey_id().to_string()),
        _ => {}
    }
    Ok(())
}

/// The failure reason the hook received, as the scenario writes one.
fn reason<H: FailureHookKind, M>(
    got: &mut Requested<'_, ScriptedWorkflow, H, M>,
    optional: bool,
) -> Result<Option<model::Reason>, Error> {
    let reason = if optional {
        got.optional_reason()?
    } else {
        Some(got.reason()?)
    };
    reason.as_ref().map(written).transpose()
}

fn written(reason: &Reason) -> Result<model::Reason, Error> {
    Ok(model::Reason {
        code: reason.code().to_owned(),
        message: reason.message().map(str::to_owned),
        details: reason.details().map(serde_json::to_value).transpose()?,
    })
}

/// Declares data from the step, of the Rust type of its type.
struct NeedingFromStep<'d, H> {
    needs: HookNeeds<H>,
    data: &'d Data,
}

impl<H: StepHookKind> ForType for NeedingFromStep<'_, H> {
    type Output = HookNeeds<H>;

    fn of<T: Storable>(self) -> HookNeeds<H> {
        if self.data.optional {
            self.needs
                .optional_from_step(&OptionalInput::<T>::new(self.data.key))
        } else {
            self.needs.from_step(&Input::<T>::new(self.data.key))
        }
    }
}

/// Declares data from the workflow, of the Rust type of its type.
struct NeedingFromWorkflow<'d, H> {
    needs: HookNeeds<H>,
    data: &'d Data,
}

impl<H: HookKind> ForType for NeedingFromWorkflow<'_, H> {
    type Output = HookNeeds<H>;

    fn of<T: Storable>(self) -> HookNeeds<H> {
        if self.data.optional {
            self.needs
                .optional_from_workflow(&OptionalInput::<T>::new(self.data.key))
        } else {
            self.needs.from_workflow(&Input::<T>::new(self.data.key))
        }
    }
}

/// Declares required data from the workflow under the key, of the Rust type of its type.
pub(crate) fn requiring_from_workflow<H: HookKind>(
    needs: HookNeeds<H>,
    (key, value_type): &(String, ValueType),
) -> HookNeeds<H> {
    let data = Data::of(key, *value_type, false);
    for_type(*value_type, NeedingFromWorkflow { needs, data: &data })
}

/// Takes data from the step, as JSON, or `None` when it was absent.
struct TakingFromStep<'g, 'a, 'd, H: HookKind, M> {
    got: &'g mut Requested<'a, ScriptedWorkflow, H, M>,
    data: &'d Data,
}

impl<H: StepHookKind, M> ForType for TakingFromStep<'_, '_, '_, H, M> {
    type Output = Result<Option<Value>, Error>;

    fn of<T: Storable>(self) -> Self::Output {
        if self.data.optional {
            self.got
                .optional_from_step(&OptionalInput::<T>::new(self.data.key))?
                .map(json)
                .transpose()
        } else {
            json(self.got.from_step(&Input::<T>::new(self.data.key))?).map(Some)
        }
    }
}

/// Takes data from the workflow, as JSON, or `None` when it was absent.
struct TakingFromWorkflow<'g, 'a, 'd, H: HookKind, M> {
    got: &'g mut Requested<'a, ScriptedWorkflow, H, M>,
    data: &'d Data,
}

impl<H: HookKind, M> ForType for TakingFromWorkflow<'_, '_, '_, H, M> {
    type Output = Result<Option<Value>, Error>;

    fn of<T: Storable>(self) -> Self::Output {
        if self.data.optional {
            self.got
                .optional_from_workflow(&OptionalInput::<T>::new(self.data.key))?
                .map(json)
                .transpose()
        } else {
            json(self.got.from_workflow(&Input::<T>::new(self.data.key))?).map(Some)
        }
    }
}

/// A hook's reporter, of either mode, as the scripts use it.
trait Emits: Send {
    fn emit(&mut self, level: Level, message: String) -> Emitted<'_>;
}

impl Emits for HookReporter<'_> {
    fn emit(&mut self, level: Level, message: String) -> Emitted<'_> {
        let emitted = match level {
            Level::Info => self.info(message),
            Level::Warning => self.warning(message),
            Level::Error => self.error(message),
        };
        Box::pin(ready(emitted))
    }
}

impl Emits for AsyncHookReporter<'_> {
    fn emit(&mut self, level: Level, message: String) -> Emitted<'_> {
        match level {
            Level::Info => Box::pin(self.info(message)),
            Level::Warning => Box::pin(self.warning(message)),
            Level::Error => Box::pin(self.error(message)),
        }
    }
}

impl OnStepSuccess<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_success(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepSuccess>,
    ) -> Result<Option<OnSuccess>, Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl OnStepFailure<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_failure(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepFailure>,
    ) -> Result<Option<FailWorkflow>, Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl OnStepRetry<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_retry(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepRetry>,
    ) -> Result<Option<FailWorkflow>, Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl OnStepAbnormalTermination<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_abnormal_termination(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepAbnormalTermination>,
    ) -> Result<Option<FailWorkflow>, Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl OnWorkflowSuccess<ScriptedWorkflow> for ScriptedPolicy {
    fn on_workflow_success(
        &self,
        got: Requested<'_, ScriptedWorkflow, WorkflowSuccess>,
    ) -> Result<(), Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl OnWorkflowFailure<ScriptedWorkflow> for ScriptedPolicy {
    fn on_workflow_failure(
        &self,
        got: Requested<'_, ScriptedWorkflow, WorkflowFailure>,
    ) -> Result<(), Error> {
        block_on(self.run(got, Requested::<_, _, Synchronous>::reporter))
    }
}

impl AsyncOnStepSuccess<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_success(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepSuccess, Asynchronous>,
    ) -> impl Future<Output = Result<Option<OnSuccess>, Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

impl AsyncOnStepFailure<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_failure(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepFailure, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

impl AsyncOnStepRetry<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_retry(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepRetry, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

impl AsyncOnStepAbnormalTermination<ScriptedWorkflow> for ScriptedPolicy {
    fn on_step_abnormal_termination(
        &self,
        got: Requested<'_, ScriptedWorkflow, StepAbnormalTermination, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

impl AsyncOnWorkflowSuccess<ScriptedWorkflow> for ScriptedPolicy {
    fn on_workflow_success(
        &self,
        got: Requested<'_, ScriptedWorkflow, WorkflowSuccess, Asynchronous>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

impl AsyncOnWorkflowFailure<ScriptedWorkflow> for ScriptedPolicy {
    fn on_workflow_failure(
        &self,
        got: Requested<'_, ScriptedWorkflow, WorkflowFailure, Asynchronous>,
    ) -> impl Future<Output = Result<(), Error>> + Send {
        self.run(got, Requested::<_, _, Asynchronous>::reporter)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn returning(hook: Hook, returned: HookReturn) -> Result<Scripts, ModelError> {
        let mut policy = model::Policy::new(hook);
        policy.hook_mut(hook).unwrap().returns = returned;
        Scripts::of("policy", &policy)
    }

    fn requesting(hook: Hook, request: HookRequest) -> Result<Scripts, ModelError> {
        let mut policy = model::Policy::new(hook);
        policy.hook_mut(hook).unwrap().requests.push(request);
        Scripts::of("policy", &policy)
    }

    #[rstest]
    #[case::step_data_from_a_workflow_hook(
        Hook::Workflow(WorkflowHook::OnWorkflowSuccess),
        HookRequest::StepData {
            key: "receipt".to_owned(),
            value_type: ValueType::String,
            optional: false,
        },
        "step data"
    )]
    #[case::the_retry_cause_from_on_step_failure(
        Hook::Step(StepHook::OnStepFailure),
        HookRequest::RetryCause,
        "the retry cause"
    )]
    #[case::the_failure_reason_from_on_step_abnormal_termination(
        Hook::Step(StepHook::OnStepAbnormalTermination),
        HookRequest::FailureReason { optional: false },
        "the failure reason"
    )]
    fn a_request_a_hook_cannot_make_is_a_case_error(
        #[case] hook: Hook,
        #[case] request: HookRequest,
        #[case] named: &str,
    ) {
        assert_eq!(
            requesting(hook, request).unwrap_err(),
            ModelError::RequestNotAllowed(hook, named.to_owned())
        );
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
