//! Policy descriptors: a policy's name, the hooks it defines with what each needs, and how to
//! build it.

use std::fmt;
use std::future::{Future, ready};
use std::marker::PhantomData;
use std::pin::Pin;

use super::{
    FailWorkflow, HookNeeds, Needs, OnStepAbnormalTermination, OnStepFailure, OnStepRetry,
    OnStepSuccess, OnSuccess, OnWorkflowFailure, OnWorkflowSuccess, PolicyName, Requested,
    StepAbnormalTermination, StepFailure, StepHook, StepRetry, StepSuccess, WorkflowFailure,
    WorkflowHook, WorkflowSuccess,
};
use crate::error::Error;
use crate::mode::Synchronous;

/// A call of a hook, which ends with what the hook returned.
pub(crate) type HookCall<'a, T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'a>>;

/// The state of a policy descriptor that defines no hook yet: it cannot be attached until it
/// defines one, since a policy defines one or more hooks.
///
/// # Examples
///
/// ```
/// use itinera::mode::Synchronous;
/// use itinera::policy::{Hookless, StepPolicyDescriptor};
///
/// struct Audit;
/// struct Orders;
///
/// let audit: StepPolicyDescriptor<Audit, Orders, Synchronous, Hookless> =
///     StepPolicyDescriptor::new("audit", || Audit);
/// ```
#[derive(Debug)]
pub enum Hookless {}

/// The state of a policy descriptor that defines at least one hook, and can be attached: the
/// default.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Synchronous;
/// use itinera::policy::{
///     Hooked, OnStepSuccess, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess,
/// };
///
/// struct Audit;
///
/// impl<W: Send + Sync + 'static> OnStepSuccess<W> for Audit {
///     fn on_step_success(
///         &self,
///         _got: Requested<'_, W, StepSuccess>,
///     ) -> Result<Option<OnSuccess>, Error> {
///         Ok(None)
///     }
/// }
///
/// struct Orders;
///
/// let audit: StepPolicyDescriptor<Audit, Orders, Synchronous, Hooked> =
///     StepPolicyDescriptor::new("audit", || Audit).on_step_success();
/// ```
#[derive(Debug)]
pub enum Hooked {}

/// How the executor calls one hook of a policy `P` of kind `H`, returning `R`.
pub(crate) type Call<P, W, H, M, R> = for<'a> fn(&'a P, Requested<'a, W, H, M>) -> HookCall<'a, R>;

/// How to build a policy.
type Factory<P> = Box<dyn Fn() -> Result<P, Error> + Send + Sync>;

/// The description of one step policy `P` of workflows `W`: its name, the step hooks it defines,
/// with what each needs, and how to build it, which may fail. It is attached to steps with
/// [`StepDescriptor::policy`].
///
/// The executor builds a new instance of the policy for every attempt of a step it is attached
/// to, right after `attempt_started`, and every hook called for that attempt uses it. If building
/// fails, the journey is aborted with `policy could not be built`.
///
/// A policy defines each hook by implementing its trait, such as [`OnStepSuccess`], and naming it
/// here, such as with [`on_step_success`](Self::on_step_success). A descriptor starts
/// [`Hookless`], and can be attached only once it names a hook. A synchronous workflow's
/// policies are synchronous; an asynchronous workflow's are made with `new_async`, with the
/// `async` feature, and define asynchronous hooks.
///
/// [`StepDescriptor::policy`]: crate::step::StepDescriptor::policy
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{OnStepSuccess, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess};
///
/// struct Audit;
///
/// impl<W: Send + Sync + 'static> OnStepSuccess<W> for Audit {
///     fn on_step_success(
///         &self,
///         got: Requested<'_, W, StepSuccess>,
///     ) -> Result<Option<OnSuccess>, Error> {
///         println!("{} succeeded", got.step_name());
///         Ok(None)
///     }
/// }
///
/// struct Orders;
///
/// let audit = StepPolicyDescriptor::<_, Orders, _, _>::new("audit", || Audit).on_step_success();
/// assert_eq!(audit.name().to_string(), "audit");
/// ```
#[derive(derive_more::Debug)]
pub struct StepPolicyDescriptor<P, W, M = Synchronous, S = Hooked> {
    name: PolicyName,
    #[debug(skip)]
    factory: Factory<P>,
    hooks: Vec<StepHook>,
    needs: StepNeeds,
    #[debug(skip)]
    calls: StepCalls<P, W, M>,
    #[debug(skip)]
    state: PhantomData<fn() -> S>,
}

/// What each step hook a policy defines needs.
#[derive(Debug, Default)]
struct StepNeeds {
    success: Needs,
    failure: Needs,
    retry: Needs,
    abnormal_termination: Needs,
}

/// How to call each step hook a policy defines.
struct StepCalls<P, W, M> {
    success: Option<Call<P, W, StepSuccess, M, Option<OnSuccess>>>,
    failure: Option<Call<P, W, StepFailure, M, Option<FailWorkflow>>>,
    retry: Option<Call<P, W, StepRetry, M, Option<FailWorkflow>>>,
    abnormal_termination: Option<Call<P, W, StepAbnormalTermination, M, Option<FailWorkflow>>>,
}

impl<P, W, M> Clone for StepCalls<P, W, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, W, M> Copy for StepCalls<P, W, M> {}

impl<P, W, M> StepPolicyDescriptor<P, W, M, Hookless> {
    pub(crate) fn with_factory(name: PolicyName, factory: Factory<P>) -> Self {
        Self {
            name,
            factory,
            hooks: Vec::new(),
            needs: StepNeeds::default(),
            calls: StepCalls {
                success: None,
                failure: None,
                retry: None,
                abnormal_termination: None,
            },
            state: PhantomData,
        }
    }
}

impl<P, W, M, S> StepPolicyDescriptor<P, W, M, S> {
    /// The policy's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::StepPolicyDescriptor;
    ///
    /// struct Audit;
    /// struct Orders;
    ///
    /// let audit = StepPolicyDescriptor::<_, Orders, _, _>::new("audit", || Audit);
    /// assert_eq!(audit.name().to_string(), "audit");
    /// ```
    pub fn name(&self) -> PolicyName {
        self.name
    }

    /// The descriptor, which now defines a hook.
    fn hooked(self) -> StepPolicyDescriptor<P, W, M> {
        StepPolicyDescriptor {
            name: self.name,
            factory: self.factory,
            hooks: self.hooks,
            needs: self.needs,
            calls: self.calls,
            state: PhantomData,
        }
    }

    fn define(&mut self, hook: StepHook) {
        if !self.hooks.contains(&hook) {
            self.hooks.push(hook);
        }
    }

    pub(crate) fn define_success(
        self,
        needs: Needs,
        call: Call<P, W, StepSuccess, M, Option<OnSuccess>>,
    ) -> StepPolicyDescriptor<P, W, M> {
        let mut hooked = self.hooked();
        hooked.define(StepHook::OnStepSuccess);
        hooked.needs.success = needs;
        hooked.calls.success = Some(call);
        hooked
    }

    pub(crate) fn define_failure(
        self,
        needs: Needs,
        call: Call<P, W, StepFailure, M, Option<FailWorkflow>>,
    ) -> StepPolicyDescriptor<P, W, M> {
        let mut hooked = self.hooked();
        hooked.define(StepHook::OnStepFailure);
        hooked.needs.failure = needs;
        hooked.calls.failure = Some(call);
        hooked
    }

    pub(crate) fn define_retry(
        self,
        needs: Needs,
        call: Call<P, W, StepRetry, M, Option<FailWorkflow>>,
    ) -> StepPolicyDescriptor<P, W, M> {
        let mut hooked = self.hooked();
        hooked.define(StepHook::OnStepRetry);
        hooked.needs.retry = needs;
        hooked.calls.retry = Some(call);
        hooked
    }

    pub(crate) fn define_abnormal_termination(
        self,
        needs: Needs,
        call: Call<P, W, StepAbnormalTermination, M, Option<FailWorkflow>>,
    ) -> StepPolicyDescriptor<P, W, M> {
        let mut hooked = self.hooked();
        hooked.define(StepHook::OnStepAbnormalTermination);
        hooked.needs.abnormal_termination = needs;
        hooked.calls.abnormal_termination = Some(call);
        hooked
    }
}

impl<P: Send + Sync + 'static, W: 'static> StepPolicyDescriptor<P, W, Synchronous, Hookless> {
    /// Describes a synchronous step policy with its name and how to build it, which cannot fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::StepPolicyDescriptor;
    ///
    /// #[derive(Default)]
    /// struct Audit;
    /// struct Orders;
    ///
    /// let audit = StepPolicyDescriptor::<_, Orders, _, _>::new("audit", Audit::default);
    /// ```
    pub fn new(
        name: impl Into<PolicyName>,
        factory: impl Fn() -> P + Send + Sync + 'static,
    ) -> Self {
        Self::fallible(name, move || Ok(factory()))
    }

    /// Describes a synchronous step policy with its name and how to build it, which may fail.
    /// A failure aborts the journey with `policy could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::StepPolicyDescriptor;
    ///
    /// struct Limits {
    ///     most: i64,
    /// }
    ///
    /// struct Orders;
    ///
    /// let limits = StepPolicyDescriptor::<_, Orders, _, _>::fallible("limits", || {
    ///     let most = std::env::var("MOST").unwrap_or_else(|_| "100".to_string()).parse()?;
    ///     Ok::<_, Error>(Limits { most })
    /// });
    /// ```
    pub fn fallible(
        name: impl Into<PolicyName>,
        factory: impl Fn() -> Result<P, Error> + Send + Sync + 'static,
    ) -> Self {
        Self::with_factory(name.into(), Box::new(factory))
    }
}

impl<P: Send + Sync + 'static, W: 'static, S> StepPolicyDescriptor<P, W, Synchronous, S> {
    /// Declares that the policy defines `on step success`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     OnStepSuccess, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess,
    /// };
    ///
    /// struct Finish;
    ///
    /// impl<W: Send + Sync + 'static> OnStepSuccess<W> for Finish {
    ///     fn on_step_success(
    ///         &self,
    ///         _got: Requested<'_, W, StepSuccess>,
    ///     ) -> Result<Option<OnSuccess>, Error> {
    ///         Ok(Some(OnSuccess::FinishWorkflow))
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let finish =
    ///     StepPolicyDescriptor::<_, Orders, _, _>::new("finish", || Finish).on_step_success();
    /// ```
    pub fn on_step_success(self) -> StepPolicyDescriptor<P, W>
    where
        P: OnStepSuccess<W>,
    {
        self.on_step_success_needing(P::needs())
    }

    /// Declares that the policy defines `on step success`, needing what is given here instead of
    /// what [`OnStepSuccess::needs`] says. One policy can then be attached to several steps,
    /// reading a different key on each.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     HookNeeds, OnStepSuccess, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess,
    /// };
    /// use itinera::step::Input;
    ///
    /// struct Audit {
    ///     key: &'static str,
    /// }
    ///
    /// impl<W: Send + Sync + 'static> OnStepSuccess<W> for Audit {
    ///     fn on_step_success(
    ///         &self,
    ///         mut got: Requested<'_, W, StepSuccess>,
    ///     ) -> Result<Option<OnSuccess>, Error> {
    ///         let amount = got.from_step(&Input::<i64>::new(self.key))?;
    ///         println!("{} moved {amount}", got.step_name());
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let audit =
    ///     StepPolicyDescriptor::<_, Orders, _, _>::new("audit", || Audit { key: "refund" })
    ///         .on_step_success_needing(HookNeeds::new().from_step(&Input::<i64>::new("refund")));
    /// ```
    pub fn on_step_success_needing(
        self,
        needs: HookNeeds<StepSuccess>,
    ) -> StepPolicyDescriptor<P, W>
    where
        P: OnStepSuccess<W>,
    {
        self.define_success(needs.into(), call_on_step_success::<P, W>)
    }

    /// Declares that the policy defines `on step failure`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     FailWorkflow, OnStepFailure, Requested, StepFailure, StepPolicyDescriptor,
    /// };
    ///
    /// struct Alarm;
    ///
    /// impl<W: Send + Sync + 'static> OnStepFailure<W> for Alarm {
    ///     fn on_step_failure(
    ///         &self,
    ///         _got: Requested<'_, W, StepFailure>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let alarm =
    ///     StepPolicyDescriptor::<_, Orders, _, _>::new("alarm", || Alarm).on_step_failure();
    /// ```
    pub fn on_step_failure(self) -> StepPolicyDescriptor<P, W>
    where
        P: OnStepFailure<W>,
    {
        self.on_step_failure_needing(P::needs())
    }

    /// Declares that the policy defines `on step failure`, needing what is given here instead of
    /// what [`OnStepFailure::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     FailWorkflow, HookNeeds, OnStepFailure, Requested, StepFailure, StepPolicyDescriptor,
    /// };
    ///
    /// struct Alarm;
    ///
    /// impl<W: Send + Sync + 'static> OnStepFailure<W> for Alarm {
    ///     fn on_step_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, StepFailure>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         println!("{} failed: {}", got.step_name(), got.reason()?.code());
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let alarm = StepPolicyDescriptor::<_, Orders, _, _>::new("alarm", || Alarm)
    ///     .on_step_failure_needing(HookNeeds::new().reason());
    /// ```
    pub fn on_step_failure_needing(
        self,
        needs: HookNeeds<StepFailure>,
    ) -> StepPolicyDescriptor<P, W>
    where
        P: OnStepFailure<W>,
    {
        self.define_failure(needs.into(), call_on_step_failure::<P, W>)
    }

    /// Declares that the policy defines `on step retry`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     FailWorkflow, OnStepRetry, Requested, StepPolicyDescriptor, StepRetry,
    /// };
    ///
    /// struct Patience;
    ///
    /// impl<W: Send + Sync + 'static> OnStepRetry<W> for Patience {
    ///     fn on_step_retry(
    ///         &self,
    ///         _got: Requested<'_, W, StepRetry>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let patience =
    ///     StepPolicyDescriptor::<_, Orders, _, _>::new("patience", || Patience).on_step_retry();
    /// ```
    pub fn on_step_retry(self) -> StepPolicyDescriptor<P, W>
    where
        P: OnStepRetry<W>,
    {
        self.on_step_retry_needing(P::needs())
    }

    /// Declares that the policy defines `on step retry`, needing what is given here instead of
    /// what [`OnStepRetry::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     FailWorkflow, HookNeeds, OnStepRetry, Requested, StepPolicyDescriptor, StepRetry,
    /// };
    ///
    /// struct Patience;
    ///
    /// impl<W: Send + Sync + 'static> OnStepRetry<W> for Patience {
    ///     fn on_step_retry(
    ///         &self,
    ///         mut got: Requested<'_, W, StepRetry>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         if let Some(reason) = got.optional_reason()? {
    ///             println!("retrying {}: {}", got.step_name(), reason.code());
    ///         }
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let patience = StepPolicyDescriptor::<_, Orders, _, _>::new("patience", || Patience)
    ///     .on_step_retry_needing(HookNeeds::new().optional_reason());
    /// ```
    pub fn on_step_retry_needing(self, needs: HookNeeds<StepRetry>) -> StepPolicyDescriptor<P, W>
    where
        P: OnStepRetry<W>,
    {
        self.define_retry(needs.into(), call_on_step_retry::<P, W>)
    }

    /// Declares that the policy defines `on step abnormal termination`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     FailWorkflow, OnStepAbnormalTermination, Requested, StepAbnormalTermination,
    ///     StepPolicyDescriptor,
    /// };
    ///
    /// struct Crashes;
    ///
    /// impl<W: Send + Sync + 'static> OnStepAbnormalTermination<W> for Crashes {
    ///     fn on_step_abnormal_termination(
    ///         &self,
    ///         _got: Requested<'_, W, StepAbnormalTermination>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let crashes = StepPolicyDescriptor::<_, Orders, _, _>::new("crashes", || Crashes)
    ///     .on_step_abnormal_termination();
    /// ```
    pub fn on_step_abnormal_termination(self) -> StepPolicyDescriptor<P, W>
    where
        P: OnStepAbnormalTermination<W>,
    {
        self.on_step_abnormal_termination_needing(P::needs())
    }

    /// Declares that the policy defines `on step abnormal termination`, needing what is given
    /// here instead of what [`OnStepAbnormalTermination::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     FailWorkflow, HookNeeds, OnStepAbnormalTermination, Requested,
    ///     StepAbnormalTermination, StepPolicyDescriptor,
    /// };
    ///
    /// struct Crashes;
    ///
    /// impl<W: Send + Sync + 'static> OnStepAbnormalTermination<W> for Crashes {
    ///     fn on_step_abnormal_termination(
    ///         &self,
    ///         mut got: Requested<'_, W, StepAbnormalTermination>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         println!("{} crashed: {}", got.step_name(), got.error()?);
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let crashes = StepPolicyDescriptor::<_, Orders, _, _>::new("crashes", || Crashes)
    ///     .on_step_abnormal_termination_needing(HookNeeds::new().error());
    /// ```
    pub fn on_step_abnormal_termination_needing(
        self,
        needs: HookNeeds<StepAbnormalTermination>,
    ) -> StepPolicyDescriptor<P, W>
    where
        P: OnStepAbnormalTermination<W>,
    {
        self.define_abnormal_termination(needs.into(), call_on_step_abnormal_termination::<P, W>)
    }
}

fn call_on_step_success<'a, P: OnStepSuccess<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, StepSuccess>,
) -> HookCall<'a, Option<OnSuccess>> {
    Box::pin(ready(policy.on_step_success(got)))
}

fn call_on_step_failure<'a, P: OnStepFailure<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, StepFailure>,
) -> HookCall<'a, Option<FailWorkflow>> {
    Box::pin(ready(policy.on_step_failure(got)))
}

fn call_on_step_retry<'a, P: OnStepRetry<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, StepRetry>,
) -> HookCall<'a, Option<FailWorkflow>> {
    Box::pin(ready(policy.on_step_retry(got)))
}

fn call_on_step_abnormal_termination<'a, P: OnStepAbnormalTermination<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, StepAbnormalTermination>,
) -> HookCall<'a, Option<FailWorkflow>> {
    Box::pin(ready(policy.on_step_abnormal_termination(got)))
}

/// A step policy attached to a step, whatever its type.
pub(crate) trait StepPolicyEntry<W, M>: Send + Sync + fmt::Debug {
    fn name(&self) -> PolicyName;

    /// The step hooks it defines, in the order they were declared.
    fn hooks(&self) -> &[StepHook];

    fn defines(&self, hook: StepHook) -> bool {
        self.hooks().contains(&hook)
    }

    /// What one of its hooks needs.
    fn needs(&self, hook: StepHook) -> &Needs;

    /// Builds an instance of the policy, for one attempt.
    fn build(&self) -> Result<Box<dyn BuiltStepPolicy<W, M>>, Error>;
}

/// An instance of a step policy, built for one attempt, whose hooks the executor calls.
pub(crate) trait BuiltStepPolicy<W, M>: Send + Sync {
    fn on_step_success<'a>(
        &'a self,
        got: Requested<'a, W, StepSuccess, M>,
    ) -> HookCall<'a, Option<OnSuccess>>;

    fn on_step_failure<'a>(
        &'a self,
        got: Requested<'a, W, StepFailure, M>,
    ) -> HookCall<'a, Option<FailWorkflow>>;

    fn on_step_retry<'a>(
        &'a self,
        got: Requested<'a, W, StepRetry, M>,
    ) -> HookCall<'a, Option<FailWorkflow>>;

    fn on_step_abnormal_termination<'a>(
        &'a self,
        got: Requested<'a, W, StepAbnormalTermination, M>,
    ) -> HookCall<'a, Option<FailWorkflow>>;
}

/// A built policy, with how to call each hook it defines.
struct Built<P, C> {
    policy: P,
    calls: C,
}

impl<P, W, M> StepPolicyEntry<W, M> for StepPolicyDescriptor<P, W, M>
where
    P: Send + Sync + 'static,
    W: 'static,
    M: 'static,
{
    fn name(&self) -> PolicyName {
        self.name
    }

    fn hooks(&self) -> &[StepHook] {
        &self.hooks
    }

    fn needs(&self, hook: StepHook) -> &Needs {
        match hook {
            StepHook::OnStepSuccess => &self.needs.success,
            StepHook::OnStepFailure => &self.needs.failure,
            StepHook::OnStepRetry => &self.needs.retry,
            StepHook::OnStepAbnormalTermination => &self.needs.abnormal_termination,
        }
    }

    fn build(&self) -> Result<Box<dyn BuiltStepPolicy<W, M>>, Error> {
        let policy = (self.factory)()?;
        Ok(Box::new(Built {
            policy,
            calls: self.calls,
        }))
    }
}

impl<P, W, M> BuiltStepPolicy<W, M> for Built<P, StepCalls<P, W, M>>
where
    P: Send + Sync + 'static,
{
    fn on_step_success<'a>(
        &'a self,
        got: Requested<'a, W, StepSuccess, M>,
    ) -> HookCall<'a, Option<OnSuccess>> {
        match self.calls.success {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }

    fn on_step_failure<'a>(
        &'a self,
        got: Requested<'a, W, StepFailure, M>,
    ) -> HookCall<'a, Option<FailWorkflow>> {
        match self.calls.failure {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }

    fn on_step_retry<'a>(
        &'a self,
        got: Requested<'a, W, StepRetry, M>,
    ) -> HookCall<'a, Option<FailWorkflow>> {
        match self.calls.retry {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }

    fn on_step_abnormal_termination<'a>(
        &'a self,
        got: Requested<'a, W, StepAbnormalTermination, M>,
    ) -> HookCall<'a, Option<FailWorkflow>> {
        match self.calls.abnormal_termination {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }
}

/// What a hook the policy does not define answers: nothing, the default.
fn nothing<'a, T: Default + Send + 'a>() -> HookCall<'a, T> {
    Box::pin(ready(Ok(T::default())))
}

/// The description of one workflow policy `P` of workflows `W`: its name, the workflow hooks it
/// defines, with what each needs, and how to build it, which may fail. It is attached to
/// workflows with [`WorkflowBuilder::policy`].
///
/// The executor builds one instance of it for each journey, before the journey starts and before
/// its dispatcher is created. If building fails, `run` refuses the journey.
///
/// A policy defines each hook by implementing its trait, such as [`OnWorkflowSuccess`], and
/// naming it here, such as with [`on_workflow_success`](Self::on_workflow_success). A descriptor
/// starts [`Hookless`], and can be attached only once it names a hook.
///
/// [`WorkflowBuilder::policy`]: crate::workflow::WorkflowBuilder::policy
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{
///     OnWorkflowFailure, Requested, WorkflowFailure, WorkflowPolicyDescriptor,
/// };
///
/// struct Apologise;
///
/// impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Apologise {
///     fn on_workflow_failure(
///         &self,
///         _got: Requested<'_, W, WorkflowFailure>,
///     ) -> Result<(), Error> {
///         Ok(())
///     }
/// }
///
/// struct Orders;
///
/// let apologise = WorkflowPolicyDescriptor::<_, Orders, _, _>::new("apologise", || Apologise)
///     .on_workflow_failure();
/// assert_eq!(apologise.name().to_string(), "apologise");
/// ```
#[derive(derive_more::Debug)]
pub struct WorkflowPolicyDescriptor<P, W, M = Synchronous, S = Hooked> {
    name: PolicyName,
    #[debug(skip)]
    factory: Factory<P>,
    hooks: Vec<WorkflowHook>,
    needs: WorkflowNeeds,
    #[debug(skip)]
    calls: WorkflowCalls<P, W, M>,
    #[debug(skip)]
    state: PhantomData<fn() -> S>,
}

/// What each workflow hook a policy defines needs.
#[derive(Debug, Default)]
struct WorkflowNeeds {
    success: Needs,
    failure: Needs,
}

/// How to call each workflow hook a policy defines.
struct WorkflowCalls<P, W, M> {
    success: Option<Call<P, W, WorkflowSuccess, M, ()>>,
    failure: Option<Call<P, W, WorkflowFailure, M, ()>>,
}

impl<P, W, M> Clone for WorkflowCalls<P, W, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, W, M> Copy for WorkflowCalls<P, W, M> {}

impl<P, W, M> WorkflowPolicyDescriptor<P, W, M, Hookless> {
    pub(crate) fn with_factory(name: PolicyName, factory: Factory<P>) -> Self {
        Self {
            name,
            factory,
            hooks: Vec::new(),
            needs: WorkflowNeeds::default(),
            calls: WorkflowCalls {
                success: None,
                failure: None,
            },
            state: PhantomData,
        }
    }
}

impl<P, W, M, S> WorkflowPolicyDescriptor<P, W, M, S> {
    /// The policy's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::WorkflowPolicyDescriptor;
    ///
    /// struct Notify;
    /// struct Orders;
    ///
    /// let notify = WorkflowPolicyDescriptor::<_, Orders, _, _>::new("notify", || Notify);
    /// assert_eq!(notify.name().to_string(), "notify");
    /// ```
    pub fn name(&self) -> PolicyName {
        self.name
    }

    /// The descriptor, which now defines a hook.
    fn hooked(self) -> WorkflowPolicyDescriptor<P, W, M> {
        WorkflowPolicyDescriptor {
            name: self.name,
            factory: self.factory,
            hooks: self.hooks,
            needs: self.needs,
            calls: self.calls,
            state: PhantomData,
        }
    }

    fn define(&mut self, hook: WorkflowHook) {
        if !self.hooks.contains(&hook) {
            self.hooks.push(hook);
        }
    }

    pub(crate) fn define_success(
        self,
        needs: Needs,
        call: Call<P, W, WorkflowSuccess, M, ()>,
    ) -> WorkflowPolicyDescriptor<P, W, M> {
        let mut hooked = self.hooked();
        hooked.define(WorkflowHook::OnWorkflowSuccess);
        hooked.needs.success = needs;
        hooked.calls.success = Some(call);
        hooked
    }

    pub(crate) fn define_failure(
        self,
        needs: Needs,
        call: Call<P, W, WorkflowFailure, M, ()>,
    ) -> WorkflowPolicyDescriptor<P, W, M> {
        let mut hooked = self.hooked();
        hooked.define(WorkflowHook::OnWorkflowFailure);
        hooked.needs.failure = needs;
        hooked.calls.failure = Some(call);
        hooked
    }
}

impl<P: Send + Sync + 'static, W: 'static> WorkflowPolicyDescriptor<P, W, Synchronous, Hookless> {
    /// Describes a synchronous workflow policy with its name and how to build it, which cannot
    /// fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::WorkflowPolicyDescriptor;
    ///
    /// #[derive(Default)]
    /// struct Notify;
    /// struct Orders;
    ///
    /// let notify = WorkflowPolicyDescriptor::<_, Orders, _, _>::new("notify", Notify::default);
    /// ```
    pub fn new(
        name: impl Into<PolicyName>,
        factory: impl Fn() -> P + Send + Sync + 'static,
    ) -> Self {
        Self::fallible(name, move || Ok(factory()))
    }

    /// Describes a synchronous workflow policy with its name and how to build it, which may
    /// fail. A failure refuses the journey before it starts.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::WorkflowPolicyDescriptor;
    ///
    /// struct Notify {
    ///     address: String,
    /// }
    ///
    /// struct Orders;
    ///
    /// let notify = WorkflowPolicyDescriptor::<_, Orders, _, _>::fallible("notify", || {
    ///     let address = std::env::var("NOTIFY").map_err(Error::from)?;
    ///     Ok(Notify { address })
    /// });
    /// ```
    pub fn fallible(
        name: impl Into<PolicyName>,
        factory: impl Fn() -> Result<P, Error> + Send + Sync + 'static,
    ) -> Self {
        Self::with_factory(name.into(), Box::new(factory))
    }
}

impl<P: Send + Sync + 'static, W: 'static, S> WorkflowPolicyDescriptor<P, W, Synchronous, S> {
    /// Declares that the policy defines `on workflow success`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     OnWorkflowSuccess, Requested, WorkflowPolicyDescriptor, WorkflowSuccess,
    /// };
    ///
    /// struct Celebrate;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Celebrate {
    ///     fn on_workflow_success(
    ///         &self,
    ///         _got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let celebrate = WorkflowPolicyDescriptor::<_, Orders, _, _>::new("celebrate", || Celebrate)
    ///     .on_workflow_success();
    /// ```
    pub fn on_workflow_success(self) -> WorkflowPolicyDescriptor<P, W>
    where
        P: OnWorkflowSuccess<W>,
    {
        self.on_workflow_success_needing(P::needs())
    }

    /// Declares that the policy defines `on workflow success`, needing what is given here
    /// instead of what [`OnWorkflowSuccess::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     HookNeeds, OnWorkflowSuccess, Requested, WorkflowPolicyDescriptor, WorkflowSuccess,
    /// };
    /// use itinera::step::Input;
    ///
    /// const TOTAL: Input<i64> = Input::new("total");
    ///
    /// struct Celebrate;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowSuccess<W> for Celebrate {
    ///     fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess>,
    ///     ) -> Result<(), Error> {
    ///         println!(
    ///             "{} came to {}",
    ///             got.journey_id(),
    ///             got.from_workflow(&TOTAL)?
    ///         );
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let celebrate = WorkflowPolicyDescriptor::<_, Orders, _, _>::new("celebrate", || Celebrate)
    ///     .on_workflow_success_needing(HookNeeds::new().from_workflow(&TOTAL));
    /// ```
    pub fn on_workflow_success_needing(
        self,
        needs: HookNeeds<WorkflowSuccess>,
    ) -> WorkflowPolicyDescriptor<P, W>
    where
        P: OnWorkflowSuccess<W>,
    {
        self.define_success(needs.into(), call_on_workflow_success::<P, W>)
    }

    /// Declares that the policy defines `on workflow failure`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     OnWorkflowFailure, Requested, WorkflowFailure, WorkflowPolicyDescriptor,
    /// };
    ///
    /// struct Apologise;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Apologise {
    ///     fn on_workflow_failure(
    ///         &self,
    ///         _got: Requested<'_, W, WorkflowFailure>,
    ///     ) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let apologise = WorkflowPolicyDescriptor::<_, Orders, _, _>::new("apologise", || Apologise)
    ///     .on_workflow_failure();
    /// ```
    pub fn on_workflow_failure(self) -> WorkflowPolicyDescriptor<P, W>
    where
        P: OnWorkflowFailure<W>,
    {
        self.on_workflow_failure_needing(P::needs())
    }

    /// Declares that the policy defines `on workflow failure`, needing what is given here
    /// instead of what [`OnWorkflowFailure::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     HookNeeds, OnWorkflowFailure, Requested, WorkflowFailure, WorkflowPolicyDescriptor,
    /// };
    ///
    /// struct Apologise;
    ///
    /// impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Apologise {
    ///     fn on_workflow_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowFailure>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?
    ///             .warning(format!("{} failed", got.journey_id()))?;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let apologise = WorkflowPolicyDescriptor::<_, Orders, _, _>::new("apologise", || Apologise)
    ///     .on_workflow_failure_needing(HookNeeds::new().reporter());
    /// ```
    pub fn on_workflow_failure_needing(
        self,
        needs: HookNeeds<WorkflowFailure>,
    ) -> WorkflowPolicyDescriptor<P, W>
    where
        P: OnWorkflowFailure<W>,
    {
        self.define_failure(needs.into(), call_on_workflow_failure::<P, W>)
    }
}

fn call_on_workflow_success<'a, P: OnWorkflowSuccess<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, WorkflowSuccess>,
) -> HookCall<'a, ()> {
    Box::pin(ready(policy.on_workflow_success(got)))
}

fn call_on_workflow_failure<'a, P: OnWorkflowFailure<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, WorkflowFailure>,
) -> HookCall<'a, ()> {
    Box::pin(ready(policy.on_workflow_failure(got)))
}

/// A workflow policy attached to a workflow, whatever its type.
pub(crate) trait WorkflowPolicyEntry<W, M>: Send + Sync + fmt::Debug {
    fn name(&self) -> PolicyName;

    /// The workflow hooks it defines, in the order they were declared.
    fn hooks(&self) -> &[WorkflowHook];

    fn defines(&self, hook: WorkflowHook) -> bool {
        self.hooks().contains(&hook)
    }

    /// What one of its hooks needs.
    fn needs(&self, hook: WorkflowHook) -> &Needs;

    /// Builds an instance of the policy, for one journey.
    fn build(&self) -> Result<Box<dyn BuiltWorkflowPolicy<W, M>>, Error>;
}

/// An instance of a workflow policy, built for one journey, whose hooks the executor calls.
pub(crate) trait BuiltWorkflowPolicy<W, M>: Send + Sync {
    fn on_workflow_success<'a>(
        &'a self,
        got: Requested<'a, W, WorkflowSuccess, M>,
    ) -> HookCall<'a, ()>;

    fn on_workflow_failure<'a>(
        &'a self,
        got: Requested<'a, W, WorkflowFailure, M>,
    ) -> HookCall<'a, ()>;
}

impl<P, W, M> WorkflowPolicyEntry<W, M> for WorkflowPolicyDescriptor<P, W, M>
where
    P: Send + Sync + 'static,
    W: 'static,
    M: 'static,
{
    fn name(&self) -> PolicyName {
        self.name
    }

    fn hooks(&self) -> &[WorkflowHook] {
        &self.hooks
    }

    fn needs(&self, hook: WorkflowHook) -> &Needs {
        match hook {
            WorkflowHook::OnWorkflowSuccess => &self.needs.success,
            WorkflowHook::OnWorkflowFailure => &self.needs.failure,
        }
    }

    fn build(&self) -> Result<Box<dyn BuiltWorkflowPolicy<W, M>>, Error> {
        let policy = (self.factory)()?;
        Ok(Box::new(Built {
            policy,
            calls: self.calls,
        }))
    }
}

impl<P, W, M> BuiltWorkflowPolicy<W, M> for Built<P, WorkflowCalls<P, W, M>>
where
    P: Send + Sync + 'static,
{
    fn on_workflow_success<'a>(
        &'a self,
        got: Requested<'a, W, WorkflowSuccess, M>,
    ) -> HookCall<'a, ()> {
        match self.calls.success {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }

    fn on_workflow_failure<'a>(
        &'a self,
        got: Requested<'a, W, WorkflowFailure, M>,
    ) -> HookCall<'a, ()> {
        match self.calls.failure {
            Some(call) => call(&self.policy, got),
            None => nothing(),
        }
    }
}
