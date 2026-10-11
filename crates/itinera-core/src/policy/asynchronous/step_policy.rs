//! The constructors and hook methods of an asynchronous workflow's step policy descriptor.

use super::{
    AsyncOnStepAbnormalTermination, AsyncOnStepFailure, AsyncOnStepRetry, AsyncOnStepSuccess,
};
use crate::error::Error;
use crate::mode::Asynchronous;
use crate::policy::{
    FailWorkflow, HookCall, HookNeeds, Hookless, OnSuccess, PolicyName, Requested,
    StepAbnormalTermination, StepFailure, StepPolicyDescriptor, StepRetry, StepSuccess,
};

impl<P: Send + Sync + 'static, W: 'static> StepPolicyDescriptor<P, W, Asynchronous, Hookless> {
    /// Describes an asynchronous workflow's step policy with its name and how to build it, which
    /// cannot fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::StepPolicyDescriptor;
    ///
    /// #[derive(Default)]
    /// struct Audit;
    /// struct Orders;
    ///
    /// let audit =
    ///     StepPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("audit", Audit::default);
    /// ```
    pub fn new_async(
        name: impl Into<PolicyName>,
        factory: impl Fn() -> P + Send + Sync + 'static,
    ) -> Self {
        Self::fallible_async(name, move || Ok(factory()))
    }

    /// Describes an asynchronous workflow's step policy with its name and how to build it, which
    /// may fail. A failure aborts the journey with `policy could not be built`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::StepPolicyDescriptor;
    ///
    /// struct Limits {
    ///     most: i64,
    /// }
    ///
    /// struct Orders;
    ///
    /// let limits =
    ///     StepPolicyDescriptor::<_, Orders, Asynchronous, _>::fallible_async("limits", || {
    ///         let most = std::env::var("MOST")
    ///             .unwrap_or_else(|_| "100".to_string())
    ///             .parse()?;
    ///         Ok::<_, Error>(Limits { most })
    ///     });
    /// ```
    pub fn fallible_async(
        name: impl Into<PolicyName>,
        factory: impl Fn() -> Result<P, Error> + Send + Sync + 'static,
    ) -> Self {
        Self::with_factory(name.into(), Box::new(factory))
    }
}

impl<P: Send + Sync + 'static, W: 'static, S> StepPolicyDescriptor<P, W, Asynchronous, S> {
    /// Declares that the policy defines `on step success`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnStepSuccess, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess,
    /// };
    ///
    /// struct Finish;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnStepSuccess<W> for Finish {
    ///     async fn on_step_success(
    ///         &self,
    ///         _got: Requested<'_, W, StepSuccess, Asynchronous>,
    ///     ) -> Result<Option<OnSuccess>, Error> {
    ///         Ok(Some(OnSuccess::FinishWorkflow))
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let finish =
    ///     StepPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("finish", || Finish)
    ///         .on_step_success();
    /// ```
    pub fn on_step_success(self) -> StepPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnStepSuccess<W>,
    {
        self.on_step_success_needing(P::needs())
    }

    /// Declares that the policy defines `on step success`, needing what is given here instead of
    /// what [`AsyncOnStepSuccess::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnStepSuccess, HookNeeds, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess,
    /// };
    /// use itinera::step::Input;
    ///
    /// struct Audit {
    ///     key: &'static str,
    /// }
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnStepSuccess<W> for Audit {
    ///     async fn on_step_success(
    ///         &self,
    ///         mut got: Requested<'_, W, StepSuccess, Asynchronous>,
    ///     ) -> Result<Option<OnSuccess>, Error> {
    ///         let amount = got.from_step(&Input::<i64>::new(self.key))?;
    ///         println!("{} moved {amount}", got.step_name());
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let audit = StepPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("audit", || {
    ///     Audit { key: "refund" }
    /// })
    /// .on_step_success_needing(HookNeeds::new().from_step(&Input::<i64>::new("refund")));
    /// ```
    pub fn on_step_success_needing(
        self,
        needs: HookNeeds<StepSuccess>,
    ) -> StepPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnStepSuccess<W>,
    {
        self.define_success(needs.into(), call_on_step_success::<P, W>)
    }

    /// Declares that the policy defines `on step failure`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnStepFailure, FailWorkflow, Requested, StepFailure, StepPolicyDescriptor,
    /// };
    ///
    /// struct Alarm;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnStepFailure<W> for Alarm {
    ///     async fn on_step_failure(
    ///         &self,
    ///         _got: Requested<'_, W, StepFailure, Asynchronous>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let alarm =
    ///     StepPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("alarm", || Alarm)
    ///         .on_step_failure();
    /// ```
    pub fn on_step_failure(self) -> StepPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnStepFailure<W>,
    {
        self.on_step_failure_needing(P::needs())
    }

    /// Declares that the policy defines `on step failure`, needing what is given here instead of
    /// what [`AsyncOnStepFailure::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnStepFailure, FailWorkflow, HookNeeds, Requested, StepFailure,
    ///     StepPolicyDescriptor,
    /// };
    ///
    /// struct Alarm;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnStepFailure<W> for Alarm {
    ///     async fn on_step_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, StepFailure, Asynchronous>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         println!("{} failed: {}", got.step_name(), got.reason()?.code());
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let alarm =
    ///     StepPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("alarm", || Alarm)
    ///         .on_step_failure_needing(HookNeeds::new().reason());
    /// ```
    pub fn on_step_failure_needing(
        self,
        needs: HookNeeds<StepFailure>,
    ) -> StepPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnStepFailure<W>,
    {
        self.define_failure(needs.into(), call_on_step_failure::<P, W>)
    }

    /// Declares that the policy defines `on step retry`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnStepRetry, FailWorkflow, Requested, StepPolicyDescriptor, StepRetry,
    /// };
    ///
    /// struct Patience;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnStepRetry<W> for Patience {
    ///     async fn on_step_retry(
    ///         &self,
    ///         _got: Requested<'_, W, StepRetry, Asynchronous>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let patience =
    ///     StepPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("patience", || Patience)
    ///         .on_step_retry();
    /// ```
    pub fn on_step_retry(self) -> StepPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnStepRetry<W>,
    {
        self.on_step_retry_needing(P::needs())
    }

    /// Declares that the policy defines `on step retry`, needing what is given here instead of
    /// what [`AsyncOnStepRetry::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnStepRetry, FailWorkflow, HookNeeds, Requested, StepPolicyDescriptor, StepRetry,
    /// };
    ///
    /// struct Patience;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnStepRetry<W> for Patience {
    ///     async fn on_step_retry(
    ///         &self,
    ///         mut got: Requested<'_, W, StepRetry, Asynchronous>,
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
    /// let patience =
    ///     StepPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("patience", || Patience)
    ///         .on_step_retry_needing(HookNeeds::new().optional_reason());
    /// ```
    pub fn on_step_retry_needing(
        self,
        needs: HookNeeds<StepRetry>,
    ) -> StepPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnStepRetry<W>,
    {
        self.define_retry(needs.into(), call_on_step_retry::<P, W>)
    }

    /// Declares that the policy defines `on step abnormal termination`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnStepAbnormalTermination, FailWorkflow, Requested, StepAbnormalTermination,
    ///     StepPolicyDescriptor,
    /// };
    ///
    /// struct Crashes;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnStepAbnormalTermination<W> for Crashes {
    ///     async fn on_step_abnormal_termination(
    ///         &self,
    ///         _got: Requested<'_, W, StepAbnormalTermination, Asynchronous>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let crashes =
    ///     StepPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("crashes", || Crashes)
    ///         .on_step_abnormal_termination();
    /// ```
    pub fn on_step_abnormal_termination(self) -> StepPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnStepAbnormalTermination<W>,
    {
        self.on_step_abnormal_termination_needing(P::needs())
    }

    /// Declares that the policy defines `on step abnormal termination`, needing what is given here
    /// instead of what [`AsyncOnStepAbnormalTermination::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnStepAbnormalTermination, FailWorkflow, HookNeeds, Requested,
    ///     StepAbnormalTermination, StepPolicyDescriptor,
    /// };
    ///
    /// struct Crashes;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnStepAbnormalTermination<W> for Crashes {
    ///     async fn on_step_abnormal_termination(
    ///         &self,
    ///         mut got: Requested<'_, W, StepAbnormalTermination, Asynchronous>,
    ///     ) -> Result<Option<FailWorkflow>, Error> {
    ///         println!("{} crashed: {}", got.step_name(), got.error()?);
    ///         Ok(None)
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let crashes =
    ///     StepPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("crashes", || Crashes)
    ///         .on_step_abnormal_termination_needing(HookNeeds::new().error());
    /// ```
    pub fn on_step_abnormal_termination_needing(
        self,
        needs: HookNeeds<StepAbnormalTermination>,
    ) -> StepPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnStepAbnormalTermination<W>,
    {
        self.define_abnormal_termination(needs.into(), call_on_step_abnormal_termination::<P, W>)
    }
}

fn call_on_step_success<'a, P: AsyncOnStepSuccess<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, StepSuccess, Asynchronous>,
) -> HookCall<'a, Option<OnSuccess>> {
    Box::pin(policy.on_step_success(got))
}

fn call_on_step_failure<'a, P: AsyncOnStepFailure<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, StepFailure, Asynchronous>,
) -> HookCall<'a, Option<FailWorkflow>> {
    Box::pin(policy.on_step_failure(got))
}

fn call_on_step_retry<'a, P: AsyncOnStepRetry<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, StepRetry, Asynchronous>,
) -> HookCall<'a, Option<FailWorkflow>> {
    Box::pin(policy.on_step_retry(got))
}

fn call_on_step_abnormal_termination<'a, P: AsyncOnStepAbnormalTermination<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, StepAbnormalTermination, Asynchronous>,
) -> HookCall<'a, Option<FailWorkflow>> {
    Box::pin(policy.on_step_abnormal_termination(got))
}
