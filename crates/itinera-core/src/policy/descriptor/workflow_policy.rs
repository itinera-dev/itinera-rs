//! The workflow policy descriptor.

use std::future::ready;
use std::marker::PhantomData;

use super::built::Built;
use super::calls::WorkflowCalls;
use super::{BuiltWorkflowPolicy, Call, Factory, HookCall, Hooked, Hookless, WorkflowPolicyEntry};
use crate::error::Error;
use crate::mode::Synchronous;
use crate::policy::{
    HookNeeds, Needs, OnWorkflowFailure, OnWorkflowSuccess, PolicyName, Requested, WorkflowFailure,
    WorkflowHook, WorkflowSuccess,
};

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
