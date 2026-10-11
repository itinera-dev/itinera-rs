//! Workflow policy types that know how a workflow describes them.

use crate::policy::{DescribedPolicy, WorkflowHook, WorkflowPolicyDescriptor};

/// A workflow policy type that knows how a workflow `W` describes it: its descriptor, with
/// every hook it defines. A workflow can then attach the policy by its type.
///
/// `#[workflow_policy]` implements it for every workflow that provides the roles its hooks
/// request.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Synchronous;
/// use itinera::policy::{
///     DescribedPolicy, OnWorkflowFailure, PolicyName, Requested, WorkflowFailure,
///     WorkflowHook, WorkflowPolicy, WorkflowPolicyDescriptor,
/// };
/// use itinera::step::{Outcome, StepDescriptor, step_name};
/// use itinera::workflow::WorkflowDescriptor;
///
/// struct Notify;
///
/// impl DescribedPolicy for Notify {
///     type Mode = Synchronous;
///
///     fn name() -> PolicyName {
///         PolicyName::from("notify")
///     }
///
///     fn build() -> Result<Self, Error> {
///         Ok(Notify)
///     }
/// }
///
/// impl<W: Send + Sync + 'static> OnWorkflowFailure<W> for Notify {
///     fn on_workflow_failure(&self, _got: Requested<'_, W, WorkflowFailure>) -> Result<(), Error> {
///         Ok(())
///     }
/// }
///
/// impl<W: Send + Sync + 'static> WorkflowPolicy<W> for Notify {
///     const HOOKS: &'static [WorkflowHook] = &[WorkflowHook::OnWorkflowFailure];
///
///     fn descriptor() -> WorkflowPolicyDescriptor<Self, W> {
///         WorkflowPolicyDescriptor::fallible(Self::name(), Self::build).on_workflow_failure()
///     }
/// }
///
/// struct Orders;
///
/// let orders = WorkflowDescriptor::<Orders>::builder("orders")
///     .step(StepDescriptor::new(step_name!("charge"), || Ok(Outcome::success())))
///     .policy(Notify::descriptor())
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a workflow policy the workflow `{W}` can attach by its type",
    label = "not a workflow policy of this workflow",
    note = "write `#[workflow_policy]` on the block that holds its hooks, and give the workflow \
            the roles they request"
)]
pub trait WorkflowPolicy<W>: DescribedPolicy {
    /// The hooks the policy defines, in the order its descriptor defines them.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{WorkflowHook, WorkflowPolicy};
    ///
    /// fn reports_failures<P: WorkflowPolicy<W>, W>() -> bool {
    ///     P::HOOKS.contains(&WorkflowHook::OnWorkflowFailure)
    /// }
    /// ```
    const HOOKS: &'static [WorkflowHook];

    /// The policy's descriptor, with its name, how it is built, and every hook it defines with
    /// what each needs.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{WorkflowPolicy, WorkflowPolicyDescriptor};
    ///
    /// fn notified<P: WorkflowPolicy<W>, W>() -> WorkflowPolicyDescriptor<P, W, P::Mode> {
    ///     P::descriptor()
    /// }
    /// ```
    fn descriptor() -> WorkflowPolicyDescriptor<Self, W, Self::Mode>;
}
