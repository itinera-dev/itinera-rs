//! The kinds of hook, which decide what a hook may request.

use std::fmt;

use crate::step::StepAttempt;

mod input_adapter;
pub(crate) mod sealed;
mod step_hook;
mod workflow_hook;

pub use input_adapter::InputAdapter;
pub use step_hook::{StepAbnormalTermination, StepFailure, StepRetry, StepSuccess};
pub use workflow_hook::{WorkflowFailure, WorkflowSuccess};

/// A kind of hook. [`HookNeeds`] and [`Requested`] offer a hook only the requests its kind may
/// make, so a request it may not make does not compile.
///
/// Only itinera's own kinds implement it: one for each of the six hooks of policies, and one for
/// input adapters.
///
/// [`HookNeeds`]: super::HookNeeds
/// [`Requested`]: super::Requested
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookKind, HookNeeds, WorkflowSuccess};
///
/// fn nothing<H: HookKind>() -> HookNeeds<H> {
///     HookNeeds::new()
/// }
///
/// let needs: HookNeeds<WorkflowSuccess> = nothing();
/// ```
pub trait HookKind: sealed::Sealed + Copy + Send + Sync + 'static {
    /// What the executor tells a hook of this kind about its call.
    #[doc(hidden)]
    type Context: Clone + fmt::Debug + Send + Sync;
}

/// A step hook: it acts on one attempt of a step, so it may request data from the step, the
/// step's name and the attempt number.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, StepHookKind, StepSuccess};
/// use itinera::step::Input;
///
/// const RECEIPT: Input<String> = Input::new("receipt");
///
/// fn the_receipt<H: StepHookKind>() -> HookNeeds<H> {
///     HookNeeds::new().from_step(&RECEIPT)
/// }
///
/// let needs: HookNeeds<StepSuccess> = the_receipt();
/// ```
pub trait StepHookKind: HookKind {
    /// The attempt the hook acts on.
    #[doc(hidden)]
    fn attempt(context: &Self::Context) -> &StepAttempt;
}

/// A step hook called because an attempt did not succeed, `on step failure` or `on step retry`:
/// it may request the failure's reason.
///
/// # Examples
///
/// ```
/// use itinera::policy::{FailureHookKind, HookNeeds, StepRetry};
///
/// fn the_reason<H: FailureHookKind>() -> HookNeeds<H> {
///     HookNeeds::new().optional_reason()
/// }
///
/// let needs: HookNeeds<StepRetry> = the_reason();
/// ```
pub trait FailureHookKind: StepHookKind {}

/// A step hook that may request the error of an abnormal termination: `on step failure`,
/// `on step retry` and `on step abnormal termination`.
///
/// # Examples
///
/// ```
/// use itinera::policy::{ErrorHookKind, HookNeeds, StepAbnormalTermination};
///
/// fn the_error<H: ErrorHookKind>() -> HookNeeds<H> {
///     HookNeeds::new().error()
/// }
///
/// let needs: HookNeeds<StepAbnormalTermination> = the_error();
/// ```
pub trait ErrorHookKind: StepHookKind {}

/// A hook of a policy, step hook or workflow hook: it may request roles, a contributor and a
/// reporter, which input adapters may not.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, PolicyHookKind, WorkflowFailure};
///
/// fn writing<H: PolicyHookKind>() -> HookNeeds<H> {
///     HookNeeds::new().contributor().reporter()
/// }
///
/// let needs: HookNeeds<WorkflowFailure> = writing();
/// ```
pub trait PolicyHookKind: HookKind {}
