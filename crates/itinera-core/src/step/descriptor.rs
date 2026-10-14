//! Step descriptors: what a workflow holds for one step.

use std::marker::PhantomData;
use std::num::NonZeroU32;

use super::{
    Attempts, DescribedRescueStep, Got, RescueFactory, Rescues, Resolved, Running, Step,
    StepFactory, StepName, StepNeeds,
};
use crate::error::Error;
use crate::mode::Synchronous;
use crate::policy::{StepPolicyDescriptor, StepPolicyEntry};

/// A synchronous step factory, whose steps run as soon as they are built.
struct Synchronously<F> {
    factory: F,
}

impl<F: StepFactory> Attempts for Synchronously<F> {
    fn attempt<'a>(&'a self, got: Got<'a>) -> Result<Running<'a>, Error> {
        let step = self.factory.build(&mut Resolved::new(got))?;
        Ok(Box::pin(std::future::ready(step.run())))
    }

    #[cfg(any(test, feature = "testing"))]
    fn build<'a>(&'a self, got: Got<'a>) -> Result<(), Error> {
        self.factory.build(&mut Resolved::new(got)).map(drop)
    }
}

/// What a workflow holds for one step: its name, what it needs, how to build it for each
/// attempt, its retry budget, whether an abnormal termination may be retried, the step policies
/// attached to it, in the order they were attached, and its rescue step, if it has one.
///
/// The retry budget is 0, an abnormal termination is not retried, and there is no rescue step,
/// unless the descriptor says otherwise.
///
/// `W` is the type of the workflows that may hold it, which only its policies see, and `M` their
/// mode: a step of a synchronous workflow comes from a [`StepFactory`].
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::{OnStepSuccess, OnSuccess, Requested, StepPolicyDescriptor, StepSuccess};
/// use itinera::step::{Outcome, StepDescriptor, step_name};
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
/// let audit = StepPolicyDescriptor::new("audit", || Audit).on_step_success();
/// let charge = StepDescriptor::<Orders>::new(step_name!("charge"), || Ok(Outcome::success()))
///     .retry_budget(2)
///     .no_retry_on_abnormal_termination()
///     .policy(audit);
/// assert_eq!(charge.name().to_string(), "charge");
/// ```
#[derive(derive_more::Debug)]
pub struct StepDescriptor<W, M = Synchronous> {
    name: StepName,
    needs: StepNeeds,
    #[debug(skip)]
    factory: Box<dyn Attempts>,
    /// How many retries the step allows after its first attempt.
    retry_budget: u16,
    /// Whether an abnormal termination spends the retry budget, as a failure does.
    retry_on_abnormal_termination: bool,
    policies: Vec<Box<dyn StepPolicyEntry<W, M>>>,
    /// The rescue step, described as a step of its own, under the name the executor gives it.
    #[debug("{:?}", rescue.as_deref().map(Self::name))]
    rescue: Option<Box<StepDescriptor<W, M>>>,
    #[debug(skip)]
    mode: PhantomData<fn() -> M>,
}

impl<W> StepDescriptor<W> {
    /// Describes a step of a synchronous workflow with its name and its factory, with no
    /// policies.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    ///
    /// struct Orders;
    ///
    /// let ship = StepDescriptor::<Orders>::new(step_name!("ship"), || Ok(Outcome::success()));
    /// ```
    pub fn new(name: StepName, factory: impl StepFactory) -> Self {
        Self::holding(name, factory.needs(), Box::new(Synchronously { factory }))
    }

    /// Gives the step a rescue step, built by the factory, replacing any it had. The rescue
    /// step's name is the step's name followed by `_rescue`.
    ///
    /// When an attempt of the step fails, the rescue step runs, and once it succeeds the step runs
    /// once more. A step with a rescue step is never retried: a workflow holding one with a retry
    /// budget is refused with `rescued step retried`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, RescueOutcome, StepDescriptor, step_name};
    ///
    /// struct Orders;
    ///
    /// let charge = StepDescriptor::<Orders>::new(step_name!("charge"), || Ok(Outcome::success()))
    ///     .rescue(|| Ok(RescueOutcome::success()));
    /// ```
    pub fn rescue(self, factory: impl RescueFactory) -> Self {
        let needs = factory.needs().into_step_needs();
        self.rescued(needs, Box::new(Rescues::new(factory)))
    }
}

impl<W, M> StepDescriptor<W, M> {
    pub(super) fn holding(name: StepName, needs: StepNeeds, factory: Box<dyn Attempts>) -> Self {
        Self {
            name,
            needs,
            factory,
            retry_budget: 0,
            retry_on_abnormal_termination: true,
            policies: Vec::new(),
            rescue: None,
            mode: PhantomData,
        }
    }

    /// Gives the step a rescue step, needing `needs` and built by `factory`, replacing any it had.
    pub(super) fn rescued(mut self, needs: StepNeeds, factory: Box<dyn Attempts>) -> Self {
        let rescue = Self::holding(self.name.rescue(), needs, factory);
        self.rescue = Some(Box::new(rescue));
        self
    }

    /// Gives the step the rescue step `R`, replacing any it had. The rescue step's name is the
    /// step's name followed by `_rescue`.
    ///
    /// When an attempt of the step fails, the rescue step runs, and once it succeeds the step runs
    /// once more. A step with a rescue step is never retried: a workflow holding one with a retry
    /// budget is refused with `rescued step retried`.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::mode::Synchronous;
    /// use itinera::step::{
    ///     DescribedRescueStep, Outcome, RescueOutcome, RescueStep, StepDescriptor, step_name,
    /// };
    ///
    /// struct RefreshToken;
    ///
    /// impl RescueStep for RefreshToken {
    ///     fn run(self) -> Result<RescueOutcome, Error> {
    ///         Self::succeed()
    ///     }
    /// }
    ///
    /// impl DescribedRescueStep for RefreshToken {
    ///     type Mode = Synchronous;
    ///
    ///     fn rescue<W>(step: StepDescriptor<W>) -> StepDescriptor<W> {
    ///         step.rescue(|| RefreshToken.run())
    ///     }
    /// }
    ///
    /// struct Orders;
    ///
    /// let charge = StepDescriptor::<Orders>::new(step_name!("charge"), || Ok(Outcome::success()))
    ///     .rescued_by::<RefreshToken>();
    /// ```
    pub fn rescued_by<R: DescribedRescueStep<Mode = M>>(self) -> Self {
        R::rescue(self)
    }

    /// Sets how many retries the step allows after its first attempt: a step with a budget of
    /// `n` is attempted at most `n + 1` times. A failure, and an abnormal termination unless the
    /// step turns off retrying it, spend the same budget. A budget of 0, the default, means the
    /// step is never retried.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    ///
    /// struct Orders;
    ///
    /// let charge = StepDescriptor::<Orders>::new(step_name!("charge"), || Ok(Outcome::success()))
    ///     .retry_budget(3);
    /// ```
    pub fn retry_budget(mut self, retries: u16) -> Self {
        self.retry_budget = retries;
        self
    }

    /// Turns off retrying the step after an abnormal termination: one gives the step up at once,
    /// whatever its retry budget. Without it, an abnormal termination is retried within the budget,
    /// as a failure is. Turn it off when an error the step did not anticipate may already have
    /// had side effects, such as a payment sent.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    ///
    /// struct Orders;
    ///
    /// let charge = StepDescriptor::<Orders>::new(step_name!("charge"), || Ok(Outcome::success()))
    ///     .retry_budget(1)
    ///     .no_retry_on_abnormal_termination();
    /// ```
    pub fn no_retry_on_abnormal_termination(mut self) -> Self {
        self.retry_on_abnormal_termination = false;
        self
    }

    /// Attaches a step policy, after those already attached.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{
    ///     FailWorkflow, OnStepFailure, OnStepSuccess, OnSuccess, Requested, StepFailure,
    ///     StepPolicyDescriptor, StepSuccess,
    /// };
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
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
    /// let charge = StepDescriptor::<Orders>::new(step_name!("charge"), || Ok(Outcome::success()))
    ///     .policy(StepPolicyDescriptor::new("audit", || Audit).on_step_success())
    ///     .policy(StepPolicyDescriptor::new("alarm", || Alarm).on_step_failure());
    /// ```
    pub fn policy<P: Send + Sync + 'static>(mut self, policy: StepPolicyDescriptor<P, W, M>) -> Self
    where
        W: 'static,
        M: 'static,
    {
        self.policies.push(Box::new(policy));
        self
    }

    /// The step's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::step::{Outcome, StepDescriptor, step_name};
    ///
    /// struct Orders;
    ///
    /// let ship = StepDescriptor::<Orders>::new(step_name!("ship"), || Ok(Outcome::success()));
    /// assert_eq!(ship.name(), step_name!("ship"));
    /// ```
    pub fn name(&self) -> StepName {
        self.name
    }

    pub(crate) fn needs(&self) -> &StepNeeds {
        &self.needs
    }

    /// How many attempts the retry budget allows: one more than the retries.
    #[cfg(any(test, feature = "testing"))]
    pub(crate) fn attempts_allowed(&self) -> NonZeroU32 {
        NonZeroU32::MIN.saturating_add(u32::from(self.retry_budget))
    }

    /// Whether the retry budget allows another attempt after this one.
    pub(crate) fn budget_allows_after(&self, attempt: NonZeroU32) -> bool {
        attempt.get() <= u32::from(self.retry_budget)
    }

    /// Whether the step has a retry budget at all.
    pub(crate) fn has_retry_budget(&self) -> bool {
        self.retry_budget > 0
    }

    pub(crate) fn retries_abnormal_termination(&self) -> bool {
        self.retry_on_abnormal_termination
    }

    pub(crate) fn policies(&self) -> &[Box<dyn StepPolicyEntry<W, M>>] {
        &self.policies
    }

    /// The step's rescue step, described as a step of its own, if it has one.
    pub(crate) fn rescue_step(&self) -> Option<&Self> {
        self.rescue.as_deref()
    }

    /// The step, then its rescue step, if it has one.
    pub(crate) fn with_rescue_step(&self) -> impl Iterator<Item = &Self> {
        std::iter::once(self).chain(self.rescue_step())
    }

    /// Whether the step allows any retry.
    pub(crate) fn allows_retries(&self) -> bool {
        self.retry_budget > 0
    }

    /// The factory that builds the step for each attempt.
    pub(crate) fn factory(&self) -> &dyn Attempts {
        self.factory.as_ref()
    }

    pub(crate) fn is_named(&self, name: StepName) -> bool {
        self.name == name
    }
}
