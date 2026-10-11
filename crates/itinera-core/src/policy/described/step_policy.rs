//! Step policy types that know how a workflow describes them.

use crate::policy::{DescribedPolicy, StepHook, StepPolicyDescriptor};

/// A step policy type that knows how a workflow `W` describes it: its descriptor, with every
/// hook it defines. A step can then attach the policy by its type.
///
/// `#[step_policy]` implements it for every workflow that provides the roles its hooks request.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Synchronous;
/// use itinera::policy::{
///     DescribedPolicy, OnStepSuccess, OnSuccess, PolicyName, Requested, StepHook, StepPolicy,
///     StepPolicyDescriptor, StepSuccess,
/// };
/// use itinera::step::{Outcome, StepDescriptor, step_name};
///
/// struct Audit;
///
/// impl DescribedPolicy for Audit {
///     type Mode = Synchronous;
///
///     fn name() -> PolicyName {
///         PolicyName::from("audit")
///     }
///
///     fn build() -> Result<Self, Error> {
///         Ok(Audit)
///     }
/// }
///
/// impl<W: Send + Sync + 'static> OnStepSuccess<W> for Audit {
///     fn on_step_success(
///         &self,
///         _got: Requested<'_, W, StepSuccess>,
///     ) -> Result<Option<OnSuccess>, Error> {
///         Self::carry_on()
///     }
/// }
///
/// impl<W: Send + Sync + 'static> StepPolicy<W> for Audit {
///     const HOOKS: &'static [StepHook] = &[StepHook::OnStepSuccess];
///
///     fn descriptor() -> StepPolicyDescriptor<Self, W> {
///         StepPolicyDescriptor::fallible(Self::name(), Self::build).on_step_success()
///     }
/// }
///
/// struct Orders;
///
/// let charge = StepDescriptor::<Orders>::new(step_name!("charge"), || Ok(Outcome::success()))
///     .policy(Audit::descriptor());
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a step policy the workflow `{W}` can attach by its type",
    label = "not a step policy of this workflow",
    note = "write `#[step_policy]` on the block that holds its hooks, and give the workflow the \
            roles they request"
)]
pub trait StepPolicy<W>: DescribedPolicy {
    /// The hooks the policy defines, in the order its descriptor defines them.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{StepHook, StepPolicy};
    ///
    /// fn retries<P: StepPolicy<W>, W>() -> bool {
    ///     P::HOOKS.contains(&StepHook::OnStepRetry)
    /// }
    /// ```
    const HOOKS: &'static [StepHook];

    /// The policy's descriptor, with its name, how it is built, and every hook it defines with
    /// what each needs.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{StepPolicy, StepPolicyDescriptor};
    ///
    /// fn audited<P: StepPolicy<W>, W>() -> StepPolicyDescriptor<P, W, P::Mode> {
    ///     P::descriptor()
    /// }
    /// ```
    fn descriptor() -> StepPolicyDescriptor<Self, W, Self::Mode>;
}
