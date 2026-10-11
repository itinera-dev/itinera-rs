//! The constructors and hook methods of an asynchronous workflow's workflow policy
//! descriptor.

use super::{AsyncOnWorkflowFailure, AsyncOnWorkflowSuccess};
use crate::error::Error;
use crate::mode::Asynchronous;
use crate::policy::{
    HookCall, HookNeeds, Hookless, PolicyName, Requested, WorkflowFailure,
    WorkflowPolicyDescriptor, WorkflowSuccess,
};

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
