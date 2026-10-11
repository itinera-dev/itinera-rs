//! Policy types that know how a workflow describes them, and the shorthands for the lifecycles
//! their step hooks return.

mod step_policy;
mod workflow_policy;

pub use step_policy::StepPolicy;
pub use workflow_policy::WorkflowPolicy;

use crate::error::Error;
use crate::mode::Mode;
use crate::policy::{FailWorkflow, OnSuccess, PolicyName};
use crate::step::Reason;

/// A policy type that knows its name and how it is built for a journey, so that a workflow can
/// attach it by its type, through [`StepPolicy`] or [`WorkflowPolicy`].
///
/// `#[step_policy]` and `#[workflow_policy]` implement it. Every policy type implementing it
/// also has the shorthands for what a step hook returns, such as `Self::carry_on()` and
/// `Self::fail_workflow("declined")`, which those macros bring into scope in the functions of
/// the block they are written on.
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::mode::Synchronous;
/// use itinera::policy::{DescribedPolicy, PolicyName};
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
/// assert_eq!(Audit::name().to_string(), "audit");
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a policy a workflow can attach by its type",
    label = "not a described policy",
    note = "write `#[step_policy]` or `#[workflow_policy]` on the block that holds its hooks"
)]
pub trait DescribedPolicy: Sized + Send + Sync + 'static {
    /// The mode of the workflows the policy can be attached to: a policy whose hooks are
    /// asynchronous is attached only to an asynchronous workflow.
    type Mode: Mode;

    /// The policy's name.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::{DescribedPolicy, PolicyName};
    ///
    /// fn name_of<P: DescribedPolicy>() -> PolicyName {
    ///     P::name()
    /// }
    /// ```
    fn name() -> PolicyName;

    /// Builds the policy for one journey.
    ///
    /// # Errors
    ///
    /// When the policy cannot be built, as when its descriptor's factory fails: a step policy
    /// aborts the journey with `policy could not be built`, and a workflow policy has the journey
    /// refused.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::DescribedPolicy;
    ///
    /// fn built<P: DescribedPolicy>() -> Result<P, Error> {
    ///     P::build()
    /// }
    /// ```
    fn build() -> Result<Self, Error>;

    /// A step hook makes no lifecycle decision: the journey goes on as it would without it.
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that a hook can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{DescribedPolicy, FailWorkflow};
    ///
    /// fn nothing_to_say<P: DescribedPolicy>() -> Result<Option<FailWorkflow>, Error> {
    ///     P::carry_on()
    /// }
    /// ```
    fn carry_on<T>() -> Result<Option<T>, Error> {
        Ok(None)
    }

    /// A step hook fails the workflow at once, for this reason. A text is the reason's code. It
    /// fits every step hook.
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that a hook can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{DescribedPolicy, FailWorkflow, OnSuccess};
    ///
    /// fn after_a_failure<P: DescribedPolicy>() -> Result<Option<FailWorkflow>, Error> {
    ///     P::fail_workflow("fraud-suspected")
    /// }
    ///
    /// fn after_a_success<P: DescribedPolicy>() -> Result<Option<OnSuccess>, Error> {
    ///     P::fail_workflow("fraud-suspected")
    /// }
    /// ```
    fn fail_workflow<T: From<FailWorkflow>>(reason: impl Into<Reason>) -> Result<Option<T>, Error> {
        Ok(Some(FailWorkflow::from(reason.into()).into()))
    }

    /// `on step success` finishes the workflow at once, skipping the steps left. It fits only
    /// that hook.
    ///
    /// # Errors
    ///
    /// Never; it returns a `Result` so that a hook can return it as it is.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::error::Error;
    /// use itinera::policy::{DescribedPolicy, OnSuccess};
    ///
    /// fn enough<P: DescribedPolicy>() -> Result<Option<OnSuccess>, Error> {
    ///     P::finish_workflow()
    /// }
    /// ```
    fn finish_workflow() -> Result<Option<OnSuccess>, Error> {
        Ok(Some(OnSuccess::FinishWorkflow))
    }
}
