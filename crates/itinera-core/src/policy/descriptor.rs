//! Policy descriptors: a policy's name, the hooks it defines with what each needs, and how to
//! build it.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use super::{Needs, PolicyName, Requested, StepHook, WorkflowHook};
use crate::error::Error;

mod built;
mod step_policy;
mod workflow_policy;

pub(crate) use built::{BuiltStepPolicy, BuiltWorkflowPolicy};
pub use step_policy::StepPolicyDescriptor;
pub use workflow_policy::WorkflowPolicyDescriptor;

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
