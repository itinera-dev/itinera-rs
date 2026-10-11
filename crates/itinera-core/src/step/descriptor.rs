//! Step descriptors: what a workflow holds for one step.

use std::marker::PhantomData;
use std::num::NonZeroU32;

use super::{Attempts, Got, Resolved, Running, Step, StepFactory, StepName, StepNeeds};
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
}

/// What a workflow holds for one step: its name, what it needs, how to build it for each
/// attempt, its retry budget, whether an abnormal termination may be retried, and the step
/// policies attached to it, in the order they were attached.
///
/// The retry budget is 0, and an abnormal termination is not retried, unless the descriptor
/// says otherwise.
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
///     .abnormal_termination_retriable()
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
    abnormal_termination_retriable: bool,
    policies: Vec<Box<dyn StepPolicyEntry<W, M>>>,
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
}

impl<W, M> StepDescriptor<W, M> {
    pub(super) fn holding(name: StepName, needs: StepNeeds, factory: Box<dyn Attempts>) -> Self {
        Self {
            name,
            needs,
            factory,
            retry_budget: 0,
            abnormal_termination_retriable: false,
            policies: Vec::new(),
            mode: PhantomData,
        }
    }

    /// Sets how many retries the step allows after its first attempt: a step with a budget of
    /// `n` is attempted at most `n + 1` times. A retriable failure, and an abnormal termination
    /// when the step allows retrying it, spend the same budget.
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

    /// Lets an abnormal termination of the step be retried, within its retry budget, as a
    /// retriable failure is. Without it, an abnormal termination gives the step up at once,
    /// since an error the step did not anticipate may already have had side effects.
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
    ///     .abnormal_termination_retriable();
    /// ```
    pub fn abnormal_termination_retriable(mut self) -> Self {
        self.abnormal_termination_retriable = true;
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

    /// Whether the retry budget allows another attempt after this one.
    pub(crate) fn budget_allows_after(&self, attempt: NonZeroU32) -> bool {
        attempt.get() <= u32::from(self.retry_budget)
    }

    pub(crate) fn retries_abnormal_termination(&self) -> bool {
        self.abnormal_termination_retriable
    }

    pub(crate) fn policies(&self) -> &[Box<dyn StepPolicyEntry<W, M>>] {
        &self.policies
    }

    /// Builds the step for one attempt and starts it, or fails to build it.
    pub(crate) fn attempt<'a>(&'a self, got: Got<'a>) -> Result<Running<'a>, Error> {
        self.factory.attempt(got)
    }

    pub(crate) fn is_named(&self, name: StepName) -> bool {
        self.name == name
    }
}
