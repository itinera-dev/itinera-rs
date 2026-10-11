//! The step policy descriptor.

use std::future::ready;
use std::marker::PhantomData;

use super::built::Built;
use super::calls::StepCalls;
use super::{BuiltStepPolicy, Call, Factory, HookCall, Hooked, Hookless, StepPolicyEntry};
use crate::error::Error;
use crate::mode::Synchronous;
use crate::policy::{
    FailWorkflow, HookNeeds, Needs, OnStepAbnormalTermination, OnStepFailure, OnStepRetry,
    OnStepSuccess, OnSuccess, PolicyName, Requested, StepAbnormalTermination, StepFailure,
    StepHook, StepRetry, StepSuccess,
};

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
