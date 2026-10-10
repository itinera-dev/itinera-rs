use std::future::Future;

use super::{
    FailWorkflow, HookCall, HookNeeds, Hookless, OnSuccess, PolicyHookKind, PolicyName, Requested,
    StepAbnormalTermination, StepFailure, StepPolicyDescriptor, StepRetry, StepSuccess,
    WorkflowFailure, WorkflowPolicyDescriptor, WorkflowSuccess,
};
use crate::error::{Error, Interrupted};
use crate::mode::Asynchronous;
use crate::step::{Level, Reporting};
use crate::value::{AnyValue, Value};

/// `on step success` of an asynchronous workflow's policy. Everything
/// [`OnStepSuccess`](super::OnStepSuccess) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{AsyncOnStepSuccess, OnSuccess, Requested, StepSuccess};
/// use itinera::step::StepName;
///
/// const CHARGE: StepName = StepName::new("charge");
///
/// struct StopAfterCharge;
///
/// impl<W: Send + Sync + 'static> AsyncOnStepSuccess<W> for StopAfterCharge {
///     async fn on_step_success(
///         &self,
///         got: Requested<'_, W, StepSuccess, Asynchronous>,
///     ) -> Result<Option<OnSuccess>, Error> {
///         Ok((got.step_name() == CHARGE).then_some(OnSuccess::FinishWorkflow))
///     }
/// }
/// ```
pub trait AsyncOnStepSuccess<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnStepSuccess, HookNeeds, StepSuccess};
    ///
    /// fn needs_of<P: AsyncOnStepSuccess<()>>() -> HookNeeds<StepSuccess> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<StepSuccess> {
        HookNeeds::new()
    }

    /// Acts on the attempt that succeeded.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{AsyncOnStepSuccess, OnSuccess, Requested, StepSuccess};
    ///
    /// async fn decide<P: AsyncOnStepSuccess<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), StepSuccess, Asynchronous>,
    /// ) -> Result<Option<OnSuccess>, Error> {
    ///     policy.on_step_success(got).await
    /// }
    /// ```
    fn on_step_success(
        &self,
        got: Requested<'_, W, StepSuccess, Asynchronous>,
    ) -> impl Future<Output = Result<Option<OnSuccess>, Error>> + Send;
}

/// `on step failure` of an asynchronous workflow's policy. Everything
/// [`OnStepFailure`](super::OnStepFailure) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{AsyncOnStepFailure, FailWorkflow, Requested, StepFailure};
///
/// struct Alarm;
///
/// impl<W: Send + Sync + 'static> AsyncOnStepFailure<W> for Alarm {
///     async fn on_step_failure(
///         &self,
///         got: Requested<'_, W, StepFailure, Asynchronous>,
///     ) -> Result<Option<FailWorkflow>, Error> {
///         println!("{} was given up", got.step_name());
///         Ok(None)
///     }
/// }
/// ```
pub trait AsyncOnStepFailure<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnStepFailure, HookNeeds, StepFailure};
    ///
    /// fn needs_of<P: AsyncOnStepFailure<()>>() -> HookNeeds<StepFailure> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<StepFailure> {
        HookNeeds::new()
    }

    /// Acts on the step that was given up.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{AsyncOnStepFailure, FailWorkflow, Requested, StepFailure};
    ///
    /// async fn decide<P: AsyncOnStepFailure<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), StepFailure, Asynchronous>,
    /// ) -> Result<Option<FailWorkflow>, Error> {
    ///     policy.on_step_failure(got).await
    /// }
    /// ```
    fn on_step_failure(
        &self,
        got: Requested<'_, W, StepFailure, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send;
}

/// `on step retry` of an asynchronous workflow's policy. Everything
/// [`OnStepRetry`](super::OnStepRetry) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{AsyncOnStepRetry, FailWorkflow, Requested, StepRetry};
///
/// struct Patience;
///
/// impl<W: Send + Sync + 'static> AsyncOnStepRetry<W> for Patience {
///     async fn on_step_retry(
///         &self,
///         got: Requested<'_, W, StepRetry, Asynchronous>,
///     ) -> Result<Option<FailWorkflow>, Error> {
///         println!("{} is attempted again", got.step_name());
///         Ok(None)
///     }
/// }
/// ```
pub trait AsyncOnStepRetry<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnStepRetry, HookNeeds, StepRetry};
    ///
    /// fn needs_of<P: AsyncOnStepRetry<()>>() -> HookNeeds<StepRetry> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<StepRetry> {
        HookNeeds::new()
    }

    /// Acts on the step before it is attempted again.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{AsyncOnStepRetry, FailWorkflow, Requested, StepRetry};
    ///
    /// async fn decide<P: AsyncOnStepRetry<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), StepRetry, Asynchronous>,
    /// ) -> Result<Option<FailWorkflow>, Error> {
    ///     policy.on_step_retry(got).await
    /// }
    /// ```
    fn on_step_retry(
        &self,
        got: Requested<'_, W, StepRetry, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send;
}

/// `on step abnormal termination` of an asynchronous workflow's policy. Everything
/// [`OnStepAbnormalTermination`](super::OnStepAbnormalTermination) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{
///     AsyncOnStepAbnormalTermination, FailWorkflow, Requested, StepAbnormalTermination,
/// };
///
/// struct Crashes;
///
/// impl<W: Send + Sync + 'static> AsyncOnStepAbnormalTermination<W> for Crashes {
///     async fn on_step_abnormal_termination(
///         &self,
///         got: Requested<'_, W, StepAbnormalTermination, Asynchronous>,
///     ) -> Result<Option<FailWorkflow>, Error> {
///         println!("{} crashed", got.step_name());
///         Ok(None)
///     }
/// }
/// ```
pub trait AsyncOnStepAbnormalTermination<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnStepAbnormalTermination, HookNeeds, StepAbnormalTermination};
    ///
    /// fn needs_of<P: AsyncOnStepAbnormalTermination<()>>() -> HookNeeds<StepAbnormalTermination> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<StepAbnormalTermination> {
        HookNeeds::new()
    }

    /// Acts on the attempt that ended in an abnormal termination.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnStepAbnormalTermination, FailWorkflow, Requested, StepAbnormalTermination,
    /// };
    ///
    /// async fn decide<P: AsyncOnStepAbnormalTermination<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), StepAbnormalTermination, Asynchronous>,
    /// ) -> Result<Option<FailWorkflow>, Error> {
    ///     policy.on_step_abnormal_termination(got).await
    /// }
    /// ```
    fn on_step_abnormal_termination(
        &self,
        got: Requested<'_, W, StepAbnormalTermination, Asynchronous>,
    ) -> impl Future<Output = Result<Option<FailWorkflow>, Error>> + Send;
}

/// `on workflow success` of an asynchronous workflow's policy. Everything
/// [`OnWorkflowSuccess`](super::OnWorkflowSuccess) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{AsyncOnWorkflowSuccess, Requested, WorkflowSuccess};
///
/// struct Celebrate;
///
/// impl<W: Send + Sync + 'static> AsyncOnWorkflowSuccess<W> for Celebrate {
///     async fn on_workflow_success(
///         &self,
///         got: Requested<'_, W, WorkflowSuccess, Asynchronous>,
///     ) -> Result<(), Error> {
///         println!("journey {} succeeded", got.journey_id());
///         Ok(())
///     }
/// }
/// ```
pub trait AsyncOnWorkflowSuccess<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnWorkflowSuccess, HookNeeds, WorkflowSuccess};
    ///
    /// fn needs_of<P: AsyncOnWorkflowSuccess<()>>() -> HookNeeds<WorkflowSuccess> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<WorkflowSuccess> {
        HookNeeds::new()
    }

    /// Acts on the journey that succeeded.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{AsyncOnWorkflowSuccess, Requested, WorkflowSuccess};
    ///
    /// async fn close<P: AsyncOnWorkflowSuccess<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), WorkflowSuccess, Asynchronous>,
    /// ) -> Result<(), Error> {
    ///     policy.on_workflow_success(got).await
    /// }
    /// ```
    fn on_workflow_success(
        &self,
        got: Requested<'_, W, WorkflowSuccess, Asynchronous>,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

/// `on workflow failure` of an asynchronous workflow's policy. Everything
/// [`OnWorkflowFailure`](super::OnWorkflowFailure) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Asynchronous;
/// use itinera::policy::{AsyncOnWorkflowFailure, Requested, WorkflowFailure};
///
/// struct Apologise;
///
/// impl<W: Send + Sync + 'static> AsyncOnWorkflowFailure<W> for Apologise {
///     async fn on_workflow_failure(
///         &self,
///         got: Requested<'_, W, WorkflowFailure, Asynchronous>,
///     ) -> Result<(), Error> {
///         println!("journey {} failed", got.journey_id());
///         Ok(())
///     }
/// }
/// ```
pub trait AsyncOnWorkflowFailure<W>: Send + Sync + 'static {
    /// What the hook needs, resolved before it is called. Nothing, unless it says otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{AsyncOnWorkflowFailure, HookNeeds, WorkflowFailure};
    ///
    /// fn needs_of<P: AsyncOnWorkflowFailure<()>>() -> HookNeeds<WorkflowFailure> {
    ///     P::needs()
    /// }
    /// ```
    fn needs() -> HookNeeds<WorkflowFailure> {
        HookNeeds::new()
    }

    /// Acts on the journey that failed.
    ///
    /// # Errors
    ///
    /// Any error aborts the journey with `hook failed`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{AsyncOnWorkflowFailure, Requested, WorkflowFailure};
    ///
    /// async fn close<P: AsyncOnWorkflowFailure<()>>(
    ///     policy: &P,
    ///     got: Requested<'_, (), WorkflowFailure, Asynchronous>,
    /// ) -> Result<(), Error> {
    ///     policy.on_workflow_failure(got).await
    /// }
    /// ```
    fn on_workflow_failure(
        &self,
        got: Requested<'_, W, WorkflowFailure, Asynchronous>,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

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

impl<P: Send + Sync + 'static, W: 'static> WorkflowPolicyDescriptor<P, W, Asynchronous, Hookless> {
    /// Describes an asynchronous workflow's workflow policy with its name and how to build it,
    /// which cannot fail.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::WorkflowPolicyDescriptor;
    ///
    /// #[derive(Default)]
    /// struct Notify;
    /// struct Orders;
    ///
    /// let notify = WorkflowPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async(
    ///     "notify",
    ///     Notify::default,
    /// );
    /// ```
    pub fn new_async(
        name: impl Into<PolicyName>,
        factory: impl Fn() -> P + Send + Sync + 'static,
    ) -> Self {
        Self::fallible_async(name, move || Ok(factory()))
    }

    /// Describes an asynchronous workflow's workflow policy with its name and how to build it,
    /// which may fail. A failure refuses the journey before it starts.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::WorkflowPolicyDescriptor;
    ///
    /// struct Notify {
    ///     address: String,
    /// }
    ///
    /// struct Orders;
    ///
    /// let notify = WorkflowPolicyDescriptor::<_, Orders, Asynchronous, _>::fallible_async(
    ///     "notify",
    ///     || {
    ///         let address = std::env::var("NOTIFY").map_err(Error::from)?;
    ///         Ok(Notify { address })
    ///     },
    /// );
    /// ```
    pub fn fallible_async(
        name: impl Into<PolicyName>,
        factory: impl Fn() -> Result<P, Error> + Send + Sync + 'static,
    ) -> Self {
        Self::with_factory(name.into(), Box::new(factory))
    }
}

impl<P: Send + Sync + 'static, W: 'static, S> WorkflowPolicyDescriptor<P, W, Asynchronous, S> {
    /// Declares that the policy defines `on workflow success`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnWorkflowSuccess, Requested, WorkflowPolicyDescriptor, WorkflowSuccess,
    /// };
    ///
    /// struct Celebrate;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnWorkflowSuccess<W> for Celebrate {
    ///     async fn on_workflow_success(
    ///         &self,
    ///         _got: Requested<'_, W, WorkflowSuccess, Asynchronous>,
    ///     ) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let celebrate =
    ///     WorkflowPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("celebrate", || {
    ///         Celebrate
    ///     })
    ///     .on_workflow_success();
    /// ```
    pub fn on_workflow_success(self) -> WorkflowPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnWorkflowSuccess<W>,
    {
        self.on_workflow_success_needing(P::needs())
    }

    /// Declares that the policy defines `on workflow success`, needing what is given here
    /// instead of what [`AsyncOnWorkflowSuccess::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnWorkflowSuccess, HookNeeds, Requested, WorkflowPolicyDescriptor, WorkflowSuccess,
    /// };
    /// use itinera::step::Input;
    ///
    /// const TOTAL: Input<i64> = Input::new("total");
    ///
    /// struct Celebrate;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnWorkflowSuccess<W> for Celebrate {
    ///     async fn on_workflow_success(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowSuccess, Asynchronous>,
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
    /// let celebrate =
    ///     WorkflowPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("celebrate", || {
    ///         Celebrate
    ///     })
    ///     .on_workflow_success_needing(HookNeeds::new().from_workflow(&TOTAL));
    /// ```
    pub fn on_workflow_success_needing(
        self,
        needs: HookNeeds<WorkflowSuccess>,
    ) -> WorkflowPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnWorkflowSuccess<W>,
    {
        self.define_success(needs.into(), call_on_workflow_success::<P, W>)
    }

    /// Declares that the policy defines `on workflow failure`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnWorkflowFailure, Requested, WorkflowFailure, WorkflowPolicyDescriptor,
    /// };
    ///
    /// struct Apologise;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnWorkflowFailure<W> for Apologise {
    ///     async fn on_workflow_failure(
    ///         &self,
    ///         _got: Requested<'_, W, WorkflowFailure, Asynchronous>,
    ///     ) -> Result<(), Error> {
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let apologise =
    ///     WorkflowPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("apologise", || {
    ///         Apologise
    ///     })
    ///     .on_workflow_failure();
    /// ```
    pub fn on_workflow_failure(self) -> WorkflowPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnWorkflowFailure<W>,
    {
        self.on_workflow_failure_needing(P::needs())
    }

    /// Declares that the policy defines `on workflow failure`, needing what is given here
    /// instead of what [`AsyncOnWorkflowFailure::needs`] says.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{
    ///     AsyncOnWorkflowFailure, HookNeeds, Requested, WorkflowFailure, WorkflowPolicyDescriptor,
    /// };
    ///
    /// struct Apologise;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnWorkflowFailure<W> for Apologise {
    ///     async fn on_workflow_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowFailure, Asynchronous>,
    ///     ) -> Result<(), Error> {
    ///         let journey = got.journey_id();
    ///         got.reporter()?.warning(format!("{journey} failed")).await?;
    ///         Ok(())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let apologise =
    ///     WorkflowPolicyDescriptor::<_, Orders, Asynchronous, _>::new_async("apologise", || {
    ///         Apologise
    ///     })
    ///     .on_workflow_failure_needing(HookNeeds::new().reporter());
    /// ```
    pub fn on_workflow_failure_needing(
        self,
        needs: HookNeeds<WorkflowFailure>,
    ) -> WorkflowPolicyDescriptor<P, W, Asynchronous>
    where
        P: AsyncOnWorkflowFailure<W>,
    {
        self.define_failure(needs.into(), call_on_workflow_failure::<P, W>)
    }
}

fn call_on_workflow_success<'a, P: AsyncOnWorkflowSuccess<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, WorkflowSuccess, Asynchronous>,
) -> HookCall<'a, ()> {
    Box::pin(policy.on_workflow_success(got))
}

fn call_on_workflow_failure<'a, P: AsyncOnWorkflowFailure<W>, W>(
    policy: &'a P,
    got: Requested<'a, W, WorkflowFailure, Asynchronous>,
) -> HookCall<'a, ()> {
    Box::pin(policy.on_workflow_failure(got))
}

impl<'a, W, H: PolicyHookKind> Requested<'a, W, H, Asynchronous> {
    /// Takes the reporter the hook declared.
    ///
    /// # Errors
    ///
    /// When the hook did not declare a reporter, or took it already.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Asynchronous;
    /// use itinera::policy::{AsyncOnWorkflowFailure, HookNeeds, Requested, WorkflowFailure};
    ///
    /// struct Alert;
    ///
    /// impl<W: Send + Sync + 'static> AsyncOnWorkflowFailure<W> for Alert {
    ///     fn needs() -> HookNeeds<WorkflowFailure> {
    ///         HookNeeds::new().reporter()
    ///     }
    ///
    ///     async fn on_workflow_failure(
    ///         &self,
    ///         mut got: Requested<'_, W, WorkflowFailure, Asynchronous>,
    ///     ) -> Result<(), Error> {
    ///         got.reporter()?.error("the order failed").await?;
    ///         Ok(())
    ///     }
    /// }
    /// ```
    pub fn reporter(&mut self) -> Result<AsyncHookReporter<'a>, Error> {
        self.take_reporting().map(AsyncHookReporter::from)
    }
}

/// An asynchronous hook's means of emitting its own events, `journey_info`, `journey_warning`
/// and `journey_error`. Everything [`HookReporter`](super::HookReporter) says applies to it.
///
/// # Examples
///
/// ```
/// use itinera::error::Interrupted;
/// use itinera::policy::AsyncHookReporter;
///
/// async fn closing(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
///     reporter.info("closing").await
/// }
/// ```
#[derive(Debug, derive_more::From)]
pub struct AsyncHookReporter<'a> {
    reporting: Reporting<'a>,
}

impl AsyncHookReporter<'_> {
    /// Emits `journey_info` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn closing(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.info("closing").await
    /// }
    /// ```
    pub fn info(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Info, message.into(), None)
    }

    /// Emits `journey_info` with a message and data, a value.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn closing(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.info_with("closing", 42_i64).await
    /// }
    /// ```
    pub fn info_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting
            .emit(Level::Info, message.into(), Some(AnyValue::new(data)))
    }

    /// Emits `journey_warning` with a message.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn late(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.warning("the order is late").await
    /// }
    /// ```
    pub fn warning(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Warning, message.into(), None)
    }

    /// Emits `journey_warning` with a message and data, a value.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn late(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.warning_with("the order is late", 42_i64).await
    /// }
    /// ```
    pub fn warning_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting
            .emit(Level::Warning, message.into(), Some(AnyValue::new(data)))
    }

    /// Emits `journey_error` with a message. It changes nothing in the journey.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn unsent(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.error("the receipt was not sent").await
    /// }
    /// ```
    pub fn error(
        &mut self,
        message: impl Into<String>,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting.emit(Level::Error, message.into(), None)
    }

    /// Emits `journey_error` with a message and data, a value. It changes nothing in the
    /// journey.
    ///
    /// # Errors
    ///
    /// [`Interrupted`] when a reporter failed, here or earlier in the hook.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Interrupted;
    /// use itinera::policy::AsyncHookReporter;
    ///
    /// async fn unsent(reporter: &mut AsyncHookReporter<'_>) -> Result<(), Interrupted> {
    ///     reporter.error_with("the receipt was not sent", 42_i64).await
    /// }
    /// ```
    pub fn error_with<T: Value>(
        &mut self,
        message: impl Into<String>,
        data: T,
    ) -> impl Future<Output = Result<(), Interrupted>> + Send + '_ {
        self.reporting
            .emit(Level::Error, message.into(), Some(AnyValue::new(data)))
    }
}
